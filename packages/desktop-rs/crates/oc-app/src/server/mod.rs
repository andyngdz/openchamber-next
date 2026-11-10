//! The sidecar's owner on the main thread: starts the CLI, records OpenCode's pids, notices a
//! CLI that exited, and stops it.
//!
//! The CLI gets `PR_SET_PDEATHSIG`, which fires when the spawning thread exits, so every start
//! runs on the main thread, the one GPUI keeps until the process ends.

mod exit;
mod failure;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use oc_sidecar::{ShutdownOutcome, Sidecar, SidecarConfig, locate_cli};
use oc_state::{ServerExit, ServerFailure};

use exit::{Liveness, liveness, server_exit};
use failure::start_failure;

/// A handle to the one sidecar of the app; clones share it.
#[derive(Clone)]
pub(crate) struct ServerProcess {
    slot: Rc<RefCell<Slot>>,
}

struct Slot {
    working_dir: PathBuf,
    sidecar: Option<Sidecar>,
}

impl ServerProcess {
    /// No server yet; [`ServerProcess::start`] runs the CLI in `working_dir`.
    pub(crate) fn new(working_dir: PathBuf) -> Self {
        let slot = Slot {
            working_dir,
            sidecar: None,
        };
        Self {
            slot: Rc::new(RefCell::new(slot)),
        }
    }

    /// Stops a server still held, then starts a new one and waits for its port, up to the
    /// sidecar's 30 s. Call on the main thread.
    pub(crate) fn start(&self) -> Result<u16, ServerFailure> {
        self.shutdown();
        let working_dir = self.slot.borrow().working_dir.clone();
        let cli = locate_cli().map_err(|error| start_failure(&error))?;
        let config = SidecarConfig::new(cli, working_dir, Vec::new());
        let sidecar = Sidecar::start(&config).map_err(|error| start_failure(&error))?;
        let port = sidecar.port();
        self.slot.borrow_mut().sidecar = Some(sidecar);
        Ok(port)
    }

    /// Records the `opencode serve` processes the CLI runs now, so shutdown stops them too.
    pub(crate) fn record_opencode_pids(&self) {
        let mut slot = self.slot.borrow_mut();
        let Some(sidecar) = slot.sidecar.as_mut() else {
            return;
        };
        let recorded = sidecar.record_opencode_pids().len();
        tracing::info!(target: crate::logging::SERVER, recorded, "[SERVER] Recorded OpenCode processes");
    }

    /// How the CLI ended, once it has. The exited CLI is reaped and each recorded OpenCode
    /// process still running is stopped; a running CLI is left alone.
    pub(crate) fn exit(&self) -> Option<ServerExit> {
        let running = self.slot.borrow().sidecar.as_ref().map(Sidecar::cli_pid)?;
        if liveness(running.as_raw()) == Liveness::Running {
            return None;
        }
        tracing::warn!(target: crate::logging::SERVER, pid = running.as_raw(), "[SERVER] Server exited while the app was running");
        let outcome = self.stop()?;
        match outcome {
            ShutdownOutcome::Exited { status, .. } | ShutdownOutcome::Killed { status, .. } => {
                Some(server_exit(status))
            }
            ShutdownOutcome::AlreadyStopped => Some(ServerExit::Unknown),
        }
    }

    /// Stops the server if one runs; a second call does nothing.
    pub(crate) fn shutdown(&self) {
        if let Some(outcome) = self.stop() {
            tracing::info!(target: crate::logging::SERVER, ?outcome, "[SERVER] Server stopped");
        }
    }

    fn stop(&self) -> Option<ShutdownOutcome> {
        let mut sidecar = self.slot.borrow_mut().sidecar.take()?;
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
