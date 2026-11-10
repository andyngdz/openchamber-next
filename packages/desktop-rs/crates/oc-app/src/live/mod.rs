//! The live server behind the window: the sidecar, the I/O runtime, and the task that talks
//! to the server.
//!
//! The runtime runs [`session::run_session`] and sends what it learns over a bounded
//! `futures` channel; a GPUI task the window owns reads that channel and applies each update
//! through `State::apply`. Requests go the other way over a second bounded channel.

mod protocol;
mod reconnect;
mod runtime;
mod session;

use futures::channel::mpsc::{self, Receiver, Sender};
use gpui_kit::Task;
use oc_client::{Client, ClientError, Directory};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub(crate) use protocol::{Command, Update};
pub(crate) use runtime::IoRuntime;

use crate::server::ServerProcess;

/// Updates in flight before the session task waits for the window: a few seconds of events.
const UPDATE_BUFFER: usize = 256;
/// Requests in flight before the window drops one.
const COMMAND_BUFFER: usize = 16;

/// One server session: the request channel, the task on the runtime, the window's reader.
struct Connection {
    commands: Sender<Command>,
    task: JoinHandle<()>,
    _updates: Task<()>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// What the window holds of the live server.
pub(crate) struct ServerLink {
    process: ServerProcess,
    runtime: Handle,
    directory: Directory,
    connection: Option<Connection>,
}

impl ServerLink {
    /// A link to `process` for `directory`, doing its I/O on `runtime`.
    pub(crate) fn new(process: ServerProcess, runtime: Handle, directory: Directory) -> Self {
        Self {
            process,
            runtime,
            directory,
            connection: None,
        }
    }

    /// The sidecar.
    pub(crate) fn process(&self) -> &ServerProcess {
        &self.process
    }

    /// Starts a session task against the server on `port`; `read_updates` starts the
    /// window's task that applies what arrives. Replaces the previous session.
    pub(crate) fn connect(
        &mut self,
        port: u16,
        read_updates: impl FnOnce(Receiver<Update>) -> Task<()>,
    ) -> Result<(), ClientError> {
        self.disconnect();
        let client = Client::new(port)?;
        let (commands, command_receiver) = mpsc::channel(COMMAND_BUFFER);
        let (update_sender, updates) = mpsc::channel(UPDATE_BUFFER);
        let task = self.runtime.spawn(session::run_session(
            client,
            self.directory.clone(),
            command_receiver,
            update_sender,
        ));
        self.connection = Some(Connection {
            commands,
            task,
            _updates: read_updates(updates),
        });
        Ok(())
    }

    /// Stops the session task and the window's reader.
    pub(crate) fn disconnect(&mut self) {
        self.connection = None;
    }

    /// Queues a request. With no session, or a full queue, the request comes back unsent.
    pub(crate) fn send(&mut self, command: Command) -> Result<(), Command> {
        let Some(connection) = self.connection.as_mut() else {
            tracing::debug!(target: crate::logging::SERVER, ?command, "[SERVER] Dropped a request with no server session");
            return Err(command);
        };
        connection.commands.try_send(command).map_err(|error| {
            tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Dropped a request");
            error.into_inner()
        })
    }
}
