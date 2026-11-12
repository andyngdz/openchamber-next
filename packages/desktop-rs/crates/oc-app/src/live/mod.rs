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
mod control;
mod execution;
mod protocol;
mod reconnect;
mod runtime;
mod send;
mod sending;
mod session;
mod startup;
mod writer;

#[cfg(test)]
#[path = "fake_server_test.rs"]
mod fake_server_test;

#[cfg(test)]
#[path = "link_test.rs"]
mod link_test;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use futures::channel::mpsc::{self, Receiver, Sender};
use gpui_kit::Task;
use oc_client::{Client, ClientError, SettingsPatch};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub use chat_folder::chat_folder_for;
pub(crate) use chat_folder::new_chat_folder;
pub(crate) use execution::{DismissCall, SendExecution, from_plan, restored_event, settled_event};
pub(crate) use protocol::{
    Command, CreateDraftPrompt, CreateSessionRequest, CreateTarget, OpenMessages, SettingsCommand,
    Update,
};
pub(crate) use runtime::IoRuntime;

use crate::server::ServerProcess;

/// Updates in flight before the session task waits for the window: a few seconds of events.
const UPDATE_BUFFER: usize = 256;
/// Requests in flight before the window drops one.
const COMMAND_BUFFER: usize = 16;
/// Settings writes in flight before the window drops one.
const SETTINGS_BUFFER: usize = 16;
/// How long a pending settings write may take before the sidecar stops or restarts.
pub(crate) const SETTINGS_FLUSH_WAIT: Duration = Duration::from_secs(3);

/// What became of a settings write handed to the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteOutcome {
    /// The writer owns it.
    Queued,
    /// There is no connection; the overlay keeps it so the next connection resends it.
    Held,
    /// The writer's queue was full; the write is abandoned.
    Dropped,
}

/// Hands `patch` to the writer behind `settings`, or reports that it must wait or is lost.
fn deliver(settings: Option<&mut Sender<SettingsCommand>>, patch: SettingsPatch) -> WriteOutcome {
    let Some(settings) = settings else {
        return WriteOutcome::Held;
    };
    match settings.try_send(SettingsCommand::Write(Box::new(patch))) {
        Ok(()) => WriteOutcome::Queued,
        Err(_) => WriteOutcome::Dropped,
    }
}

/// One server session: the request channels, the tasks on the runtime, the window's reader.
struct Connection {
    commands: Sender<Command>,
    settings: Sender<SettingsCommand>,
    task: JoinHandle<()>,
    settings_task: JoinHandle<()>,
    /// The server answered 404 for the control stream, so its queue and modes stay off.
    control_missing: bool,
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
    /// `read_updates` starts the window's task that applies what arrives. `overlay` is the
    /// local settings intent a new connection resends as its first write. Replaces the
    /// previous session.
    pub(crate) fn connect(
        &mut self,
        port: u16,
        overlay: Option<SettingsPatch>,
        read_updates: impl FnOnce(Receiver<Update>) -> Task<()>,
    ) -> Result<(), ClientError> {
        self.disconnect();
        let client = Client::new(port)?;
        let (commands, command_receiver) = mpsc::channel(COMMAND_BUFFER);
        let (mut settings, settings_receiver) = mpsc::channel(SETTINGS_BUFFER);
        if let Some(patch) = overlay {
            // The writer starts empty, so the whole overlay is one write.
            deliver(Some(&mut settings), patch);
        }
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
            control_missing: false,
            _updates: read_updates(updates),
        });
        Ok(())
    }

    /// Stops the session tasks and the window's reader.
    pub(crate) fn disconnect(&mut self) {
        self.connection.borrow_mut().take();
    }

    /// Records that the server has no `/api/openchamber/events`, so the window shows no queue
    /// strip and no shield modes and keeps sending plain prompts.
    pub(crate) fn mark_control_missing(&self) {
        if let Some(connection) = self.connection.borrow_mut().as_mut() {
            connection.control_missing = true;
        }
    }

    /// Whether the server answered 404 for the control stream.
    pub(crate) fn control_stream_missing(&self) -> bool {
        self.connection
            .borrow()
            .as_ref()
            .is_some_and(|connection| connection.control_missing)
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

    /// Queues a partial settings write for the debounced writer, or reports that it must wait
    /// for the next connection or was abandoned.
    pub(crate) fn write_settings(&self, patch: SettingsPatch) -> WriteOutcome {
        let outcome = {
            let mut connection = self.connection.borrow_mut();
            let settings = connection
                .as_mut()
                .map(|connection| &mut connection.settings);
            deliver(settings, patch)
        };
        match outcome {
            WriteOutcome::Queued => {}
            WriteOutcome::Held => tracing::debug!(
                target: crate::logging::SERVER,
                "[SERVER] Held a settings write for the next connection"
            ),
            WriteOutcome::Dropped => tracing::warn!(
                target: crate::logging::SERVER,
                "[SERVER] Dropped a settings write"
            ),
        }
        outcome
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
