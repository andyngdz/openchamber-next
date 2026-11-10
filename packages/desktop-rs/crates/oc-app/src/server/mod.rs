//! The sidecar's owner: a dedicated thread that starts the CLI, records OpenCode's pids,
//! notices a CLI that exited, and stops it, while the window stays responsive.
//!
//! The CLI gets `PR_SET_PDEATHSIG`, which fires when the spawning thread exits, so every start
//! runs on this one thread, and the thread ends only after it has stopped the sidecar. The
//! window talks to it over a channel; answers come back on oneshot channels it awaits.

mod exit;
mod failure;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc as std_mpsc;
use std::thread::{self, JoinHandle};

use futures::channel::oneshot;
use oc_sidecar::{CliState, ShutdownOutcome, Sidecar, SidecarConfig, locate_cli};
use oc_state::{ServerExit, ServerFailure};

use crate::errors::AppError;
use exit::server_exit;
use failure::start_failure;

const THREAD_NAME: &str = "oc-server";

/// What the window asks of the server thread.
enum Request {
    /// Stop a server still held, start a new one and answer with its port.
    Start(oneshot::Sender<Result<u16, ServerFailure>>),
    /// Record the `opencode serve` processes the CLI runs now.
    RecordOpencodePids,
    /// Answer how the CLI ended, once it has.
    Exit(oneshot::Sender<Option<ServerExit>>),
    /// Stop the server and end the thread.
    Shutdown,
}

/// A handle to the one sidecar of the app and the thread that owns it; clones share both.
#[derive(Clone)]
pub(crate) struct ServerProcess {
    requests: std_mpsc::Sender<Request>,
    thread: Rc<RefCell<Option<JoinHandle<()>>>>,
}

impl ServerProcess {
    /// Starts the server thread; [`ServerProcess::start`] runs the CLI in `working_dir`.
    pub(crate) fn spawn(working_dir: PathBuf) -> Result<Self, AppError> {
        let (requests, received) = std_mpsc::channel();
        let thread = thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || serve(working_dir, &received))
            .map_err(AppError::StartServerThread)?;
        Ok(Self {
            requests,
            thread: Rc::new(RefCell::new(Some(thread))),
        })
    }

    /// Stops a server still held, then starts a new one. The answer arrives once the port
    /// line does, up to the sidecar's 30 s.
    pub(crate) fn start(&self) -> oneshot::Receiver<Result<u16, ServerFailure>> {
        let (answer, answered) = oneshot::channel();
        self.ask(Request::Start(answer));
        answered
    }

    /// Records the `opencode serve` processes the CLI runs now, so shutdown stops them too.
    pub(crate) fn record_opencode_pids(&self) {
        self.ask(Request::RecordOpencodePids);
    }

    /// How the CLI ended, once it has. The exited CLI is reaped and each recorded OpenCode
    /// process still running is stopped; a running CLI is left alone and answers `None`.
    pub(crate) fn exit(&self) -> oneshot::Receiver<Option<ServerExit>> {
        let (answer, answered) = oneshot::channel();
        self.ask(Request::Exit(answer));
        answered
    }

    /// Stops the server if one runs and waits for the thread to end; a second call does
    /// nothing. It blocks for up to the sidecar's grace period, and for a start in progress.
    pub(crate) fn stop(&self) {
        let Some(thread) = self.thread.borrow_mut().take() else {
            return;
        };
        self.ask(Request::Shutdown);
        if thread.join().is_err() {
            tracing::error!(target: crate::logging::SERVER, "[SERVER] Server thread panicked");
        }
    }

    /// A request to a thread that has ended is dropped; its answer channel then reports
    /// cancellation to whoever awaits it.
    fn ask(&self, request: Request) {
        if self.requests.send(request).is_err() {
            tracing::warn!(target: crate::logging::SERVER, "[SERVER] Server thread is gone");
        }
    }
}

/// The server thread: answers requests until asked to stop or until every handle is gone,
/// then stops the sidecar before it ends.
fn serve(working_dir: PathBuf, requests: &std_mpsc::Receiver<Request>) {
    let mut owner = Owner {
        working_dir,
        sidecar: None,
    };
    while let Ok(request) = requests.recv() {
        match request {
            Request::Start(answer) => {
                answer.send(owner.start()).ok();
            }
            Request::RecordOpencodePids => owner.record_opencode_pids(),
            Request::Exit(answer) => {
                answer.send(owner.exit()).ok();
            }
            Request::Shutdown => break,
        }
    }
    owner.shutdown();
}

/// The sidecar as the server thread holds it.
struct Owner {
    working_dir: PathBuf,
    sidecar: Option<Sidecar>,
}

impl Owner {
    fn start(&mut self) -> Result<u16, ServerFailure> {
        self.shutdown();
        let cli = locate_cli().map_err(|error| start_failure(&error))?;
        let config = SidecarConfig::new(cli, self.working_dir.clone(), Vec::new());
        let sidecar = Sidecar::start(&config).map_err(|error| start_failure(&error))?;
        let port = sidecar.port();
        self.sidecar = Some(sidecar);
        Ok(port)
    }

    fn record_opencode_pids(&mut self) {
        let Some(sidecar) = self.sidecar.as_mut() else {
            return;
        };
        let recorded = sidecar.record_opencode_pids().len();
        tracing::info!(target: crate::logging::SERVER, recorded, "[SERVER] Recorded OpenCode processes");
    }

    /// A wait error never ends a session: the CLI then counts as running.
    fn exit(&mut self) -> Option<ServerExit> {
        let sidecar = self.sidecar.as_mut()?;
        let pid = sidecar.cli_pid().as_raw();
        let status = match sidecar.try_exit_status() {
            Ok(CliState::Exited(status)) => status,
            Ok(CliState::Running | CliState::Stopped) => return None,
            Err(error) => {
                tracing::warn!(target: crate::logging::SERVER, pid, %error, "[SERVER] Could not check whether the server runs");
                return None;
            }
        };
        tracing::warn!(target: crate::logging::SERVER, pid, "[SERVER] Server exited while the app was running");
        self.stop();
        Some(server_exit(status))
    }

    fn shutdown(&mut self) {
        if let Some(outcome) = self.stop() {
            tracing::info!(target: crate::logging::SERVER, ?outcome, "[SERVER] Server stopped");
        }
    }

    fn stop(&mut self) -> Option<ShutdownOutcome> {
        let mut sidecar = self.sidecar.take()?;
        match sidecar.shutdown() {
            Ok(outcome) => Some(outcome),
            Err(error) => {
                // Dropping the sidecar tries the same shutdown once more.
                tracing::error!(target: crate::logging::SERVER, %error, "[SERVER] Server shutdown failed");
                None
            }
        }
    }
}
