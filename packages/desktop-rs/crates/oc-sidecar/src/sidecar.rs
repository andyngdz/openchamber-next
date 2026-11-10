use std::process::{Child, ExitStatus};
use std::time::Duration;

use nix::unistd::Pid;

use crate::config::SidecarConfig;
use crate::errors::SidecarError;
use crate::launch::launch;
use crate::port::{is_port_in_use_message, pick_free_port};
use crate::proc_scan::RecordedPid;
use crate::shutdown::{ShutdownOutcome, child_pid, record_opencode_children, stop_cli};

/// Start attempts in total when another process takes the picked port.
pub const MAX_START_ATTEMPTS: u32 = 3;

/// What [`Sidecar::try_exit_status`] saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliState {
    /// The CLI is running.
    Running,
    /// The CLI exited on its own, with this status.
    Exited(ExitStatus),
    /// [`Sidecar::shutdown`] already stopped it.
    Stopped,
}

/// A running `openchamber serve` process and the OpenCode processes it manages.
///
/// Dropping it runs [`Sidecar::shutdown`].
#[derive(Debug)]
pub struct Sidecar {
    child: Option<Child>,
    cli_pid: Pid,
    port: u16,
    opencode: Vec<RecordedPid>,
    graceful_timeout: Duration,
}

impl Sidecar {
    /// Starts the CLI and waits for its port line, retrying on a new port when
    /// another process took the picked one, at most [`MAX_START_ATTEMPTS`]
    /// times. Other failures are not retried.
    ///
    /// The child gets `PR_SET_PDEATHSIG(SIGTERM)`, which fires when the
    /// spawning thread exits, not when the process does: the spawning thread
    /// must live as long as the sidecar should; a dedicated thread owned by
    /// the app works.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use oc_sidecar::{Sidecar, SidecarConfig, locate_cli};
    /// let config = SidecarConfig::new(locate_cli()?, "/tmp".into(), Vec::new());
    /// let mut sidecar = Sidecar::start(&config)?;
    /// println!("server on {}", sidecar.port());
    /// sidecar.shutdown()?;
    /// # Ok::<(), oc_sidecar::SidecarError>(())
    /// ```
    pub fn start(config: &SidecarConfig) -> Result<Self, SidecarError> {
        let mut attempt = 1;
        loop {
            let port = pick_free_port()?;
            match launch(config, port) {
                Ok(launched) => {
                    tracing::info!(
                        target: crate::logging::SIDECAR,
                        port = launched.port,
                        pid = launched.child.id(),
                        "[SIDECAR] Server started"
                    );
                    return Ok(Self {
                        cli_pid: child_pid(&launched.child),
                        child: Some(launched.child),
                        port: launched.port,
                        opencode: Vec::new(),
                        graceful_timeout: config.graceful_timeout,
                    });
                }
                Err(SidecarError::ExitedEarly { diagnostics })
                    if is_port_in_use_message(&diagnostics.stderr_tail)
                        || is_port_in_use_message(&diagnostics.log_tail) =>
                {
                    if attempt >= MAX_START_ATTEMPTS {
                        return Err(SidecarError::PortInUse {
                            attempts: attempt,
                            diagnostics,
                        });
                    }
                    tracing::warn!(
                        target: crate::logging::SIDECAR,
                        port,
                        attempt,
                        "[SIDECAR] Picked port was taken, retrying"
                    );
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// The port the server listens on at `127.0.0.1`.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The pid of the CLI process.
    pub fn cli_pid(&self) -> Pid {
        self.cli_pid
    }

    /// OpenCode pids recorded so far.
    pub fn opencode_pids(&self) -> Vec<Pid> {
        self.opencode.iter().map(|process| process.pid).collect()
    }

    /// Whether the CLI is still running, checked without waiting.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use oc_sidecar::{CliState, Sidecar, SidecarConfig, locate_cli};
    /// # let config = SidecarConfig::new(locate_cli()?, "/tmp".into(), Vec::new());
    /// let mut sidecar = Sidecar::start(&config)?;
    /// assert_eq!(sidecar.try_exit_status()?, CliState::Running);
    /// # Ok::<(), oc_sidecar::SidecarError>(())
    /// ```
    pub fn try_exit_status(&mut self) -> Result<CliState, SidecarError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(CliState::Stopped);
        };
        match child.try_wait().map_err(SidecarError::Wait)? {
            Some(status) => Ok(CliState::Exited(status)),
            None => Ok(CliState::Running),
        }
    }

    /// Records the `opencode serve` processes the CLI runs now, adding them to
    /// the ones recorded before, and returns all of them. Shutdown records
    /// them too, right before SIGTERM.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use oc_sidecar::{Sidecar, SidecarConfig, locate_cli};
    /// # let config = SidecarConfig::new(locate_cli()?, "/tmp".into(), Vec::new());
    /// let mut sidecar = Sidecar::start(&config)?;
    /// let pids = sidecar.record_opencode_pids();
    /// assert!(pids.iter().all(|pid| pid.as_raw() > 0));
    /// # Ok::<(), oc_sidecar::SidecarError>(())
    /// ```
    pub fn record_opencode_pids(&mut self) -> Vec<Pid> {
        if self.child.is_some() {
            record_opencode_children(self.cli_pid, &mut self.opencode);
        }
        self.opencode_pids()
    }

    /// Stops the server: SIGTERM, up to the grace period for the CLI to stop
    /// OpenCode and exit, then SIGKILL to the CLI. Afterwards each recorded
    /// OpenCode process still running gets SIGTERM, which also covers a CLI
    /// that died before shutdown. A second call returns
    /// [`ShutdownOutcome::AlreadyStopped`].
    ///
    /// OpenCode outlives a CLI killed from outside, and only recorded pids are
    /// cleaned up, so call [`Sidecar::record_opencode_pids`] once OpenCode is
    /// up; it starts shortly after the port line.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use oc_sidecar::{Sidecar, SidecarConfig, ShutdownOutcome, locate_cli};
    /// # let config = SidecarConfig::new(locate_cli()?, "/tmp".into(), Vec::new());
    /// let mut sidecar = Sidecar::start(&config)?;
    /// sidecar.shutdown()?;
    /// assert_eq!(sidecar.shutdown()?, ShutdownOutcome::AlreadyStopped);
    /// # Ok::<(), oc_sidecar::SidecarError>(())
    /// ```
    pub fn shutdown(&mut self) -> Result<ShutdownOutcome, SidecarError> {
        let Some(mut child) = self.child.take() else {
            return Ok(ShutdownOutcome::AlreadyStopped);
        };
        let outcome = stop_cli(&mut child, &mut self.opencode, self.graceful_timeout);
        if outcome.is_err() {
            self.child = Some(child);
        }
        outcome
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            tracing::error!(
                target: crate::logging::SIDECAR,
                pid = self.cli_pid.as_raw(),
                error = %error,
                "[SIDECAR] Server shutdown failed"
            );
        }
    }
}

#[cfg(test)]
#[path = "sidecar_test.rs"]
mod sidecar_test;
