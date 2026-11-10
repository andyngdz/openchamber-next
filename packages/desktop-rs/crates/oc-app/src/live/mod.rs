//! The live server behind the window: the sidecar, the I/O runtime, and the tasks that talk
//! to the server.
//!
//! The runtime runs [`session::run_session`] and [`writer::run_settings`] and sends what
//! they learn over a bounded `futures` channel; a GPUI task the window owns reads that
//! channel and applies each update through `State::apply`, `apply_sidebar` or
//! `apply_selection`. Requests go the other way over two bounded channels: one for server
//! calls and one for settings writes, which the writer debounces.

mod catalog;
mod chat_folder;
mod protocol;
mod reconnect;
mod runtime;
mod send;
mod session;
mod startup;
mod writer;

#[cfg(test)]
#[path = "fake_server_test.rs"]
mod fake_server_test;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use futures::channel::mpsc::{self, Receiver, Sender};
use gpui_kit::Task;
use oc_client::{Client, ClientError, SettingsPatch};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub(crate) use chat_folder::new_chat_folder;
pub(crate) use protocol::{
    Command, CreateDraftPrompt, CreateSessionRequest, CreateTarget, OpenMessages, SendRequest,
    SettingsCommand, Update,
};
pub(crate) use runtime::IoRuntime;

use crate::server::ServerProcess;

/// Updates in flight before the session task waits for the window: a few seconds of events.
const UPDATE_BUFFER: usize = 256;
/// Requests in flight before the window drops one.
const COMMAND_BUFFER: usize = 16;
/// Settings writes in flight before the window drops one.
const SETTINGS_BUFFER: usize = 16;

/// One server session: the request channels, the tasks on the runtime, the window's reader.
struct Connection {
    commands: Sender<Command>,
    settings: Sender<SettingsCommand>,
    task: JoinHandle<()>,
    settings_task: JoinHandle<()>,
    _updates: Task<()>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.task.abort();
        self.settings_task.abort();
    }
}

/// What the window holds of the live server.
#[derive(Clone)]
pub(crate) struct ServerLink {
    process: ServerProcess,
    runtime: Handle,
    connection: Rc<RefCell<Option<Connection>>>,
}

impl ServerLink {
    /// A link to `process`, doing its I/O on `runtime`.
    pub(crate) fn new(process: ServerProcess, runtime: Handle) -> Self {
        Self {
            process,
            runtime,
            connection: Rc::new(RefCell::new(None)),
        }
    }

    /// The sidecar.
    pub(crate) fn process(&self) -> &ServerProcess {
        &self.process
    }

    /// Starts the session and settings-writer tasks against the server on `port`;
    /// `read_updates` starts the window's task that applies what arrives. Replaces the
    /// previous session.
    pub(crate) fn connect(
        &mut self,
        port: u16,
        read_updates: impl FnOnce(Receiver<Update>) -> Task<()>,
    ) -> Result<(), ClientError> {
        self.disconnect();
        let client = Client::new(port)?;
        let (commands, command_receiver) = mpsc::channel(COMMAND_BUFFER);
        let (settings, settings_receiver) = mpsc::channel(SETTINGS_BUFFER);
        let (update_sender, updates) = mpsc::channel(UPDATE_BUFFER);
        let task = self.runtime.spawn(session::run_session(
            client.clone(),
            command_receiver,
            update_sender.clone(),
        ));
        let settings_task = self.runtime.spawn(writer::run_settings(
            client,
            settings_receiver,
            update_sender,
        ));
        *self.connection.borrow_mut() = Some(Connection {
            commands,
            settings,
            task,
            settings_task,
            _updates: read_updates(updates),
        });
        Ok(())
    }

    /// Stops the session tasks and the window's reader.
    pub(crate) fn disconnect(&mut self) {
        self.connection.borrow_mut().take();
    }

    /// Queues a request. With no session, or a full queue, the request comes back unsent.
    pub(crate) fn send(&self, command: Command) -> Result<(), Command> {
        let mut connection = self.connection.borrow_mut();
        let Some(connection) = connection.as_mut() else {
            tracing::debug!(target: crate::logging::SERVER, "[SERVER] Dropped a request with no server session");
            return Err(command);
        };
        connection.commands.try_send(command).map_err(|error| {
            tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Dropped a request");
            error.into_inner()
        })
    }

    /// Queues a partial settings write for the debounced writer.
    pub(crate) fn write_settings(&self, patch: SettingsPatch) {
        let mut connection = self.connection.borrow_mut();
        let Some(connection) = connection.as_mut() else {
            tracing::debug!(target: crate::logging::SERVER, "[SERVER] Dropped a settings write with no server session");
            return;
        };
        if let Err(error) = connection
            .settings
            .try_send(SettingsCommand::Write(Box::new(patch)))
        {
            tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Dropped a settings write");
        }
    }

    /// Waits, up to `limit`, for the writer to send what is pending; called before the
    /// sidecar stops so the last choice is not lost.
    pub(crate) fn flush_settings(&self, limit: Duration) {
        let (done, finished) = std::sync::mpsc::channel();
        let sent = {
            let mut connection = self.connection.borrow_mut();
            match connection.as_mut() {
                Some(connection) => connection
                    .settings
                    .try_send(SettingsCommand::Flush(done))
                    .is_ok(),
                None => false,
            }
        };
        if !sent {
            tracing::debug!(target: crate::logging::SERVER, "[SERVER] No settings writer to flush");
            return;
        }
        if finished.recv_timeout(limit).is_err() {
            tracing::warn!(target: crate::logging::SERVER, "[SERVER] Settings flush did not finish before the window closed");
        }
    }
}
