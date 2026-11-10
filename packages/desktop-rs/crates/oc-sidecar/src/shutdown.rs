use std::process::{Child, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

use crate::errors::SidecarError;
use crate::proc_scan::{RecordedPid, opencode_children};

const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// How the CLI stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// The sidecar was already shut down.
    AlreadyStopped,
    /// The CLI exited on its own or after SIGTERM, within the grace period.
    Exited {
        /// The CLI's exit status.
        status: ExitStatus,
        /// Recorded OpenCode pids still running after the CLI exited, which
        /// got SIGTERM. Empty when the CLI stopped OpenCode itself; set when
        /// the CLI died before shutdown, for example from an outside SIGKILL.
        terminated_opencode: Vec<Pid>,
    },
    /// The CLI outlived the grace period and got SIGKILL; each recorded
    /// OpenCode process still running got SIGTERM.
    Killed {
        /// The CLI's exit status after SIGKILL.
        status: ExitStatus,
        /// OpenCode pids that were still running and got SIGTERM.
        terminated_opencode: Vec<Pid>,
    },
}

/// The pid of a spawned child. Linux pids fit in `i32`.
pub(crate) fn child_pid(child: &Child) -> Pid {
    Pid::from_raw(child.id().cast_signed())
}

/// Adds the CLI's current `opencode serve` children to `recorded`.
pub(crate) fn record_opencode_children(cli_pid: Pid, recorded: &mut Vec<RecordedPid>) {
    for process in opencode_children(cli_pid) {
        if !recorded.contains(&process) {
            recorded.push(process);
        }
    }
}

/// Stops the CLI: records its OpenCode children, sends SIGTERM, waits up to
/// `graceful_timeout`, then sends SIGKILL. Once the CLI is gone, by any path,
/// every recorded OpenCode process still running gets SIGTERM, since OpenCode
/// runs in its own process group and the CLI's death does not reach it.
pub(crate) fn stop_cli(
    child: &mut Child,
    recorded: &mut Vec<RecordedPid>,
    graceful_timeout: Duration,
) -> Result<ShutdownOutcome, SidecarError> {
    let cli_pid = child_pid(child);
    if let Some(status) = child.try_wait().map_err(SidecarError::Wait)? {
        return exited(status, recorded);
    }
    record_opencode_children(cli_pid, recorded);
    send_signal(cli_pid, Signal::SIGTERM)?;
    if let Some(status) = wait_for_exit(child, graceful_timeout)? {
        return exited(status, recorded);
    }
    tracing::warn!(
        target: crate::logging::SIDECAR,
        pid = cli_pid.as_raw(),
        "[SIDECAR] Server ignored SIGTERM, sending SIGKILL"
    );
    send_signal(cli_pid, Signal::SIGKILL)?;
    let status = child.wait().map_err(SidecarError::Wait)?;
    let terminated_opencode = terminate_survivors(recorded)?;
    Ok(ShutdownOutcome::Killed {
        status,
        terminated_opencode,
    })
}

fn exited(status: ExitStatus, recorded: &[RecordedPid]) -> Result<ShutdownOutcome, SidecarError> {
    Ok(ShutdownOutcome::Exited {
        status,
        terminated_opencode: terminate_survivors(recorded)?,
    })
}

/// SIGTERM to each recorded process that is still the one recorded.
fn terminate_survivors(recorded: &[RecordedPid]) -> Result<Vec<Pid>, SidecarError> {
    let survivors: Vec<Pid> = recorded
        .iter()
        .filter(|process| process.is_still_running())
        .map(|process| process.pid)
        .collect();
    for pid in &survivors {
        send_signal(*pid, Signal::SIGTERM)?;
    }
    Ok(survivors)
}

/// Waits up to `timeout` for the child to exit; `None` when it is still running.
pub(crate) fn wait_for_exit(
    child: &mut Child,
    timeout: Duration,
) -> Result<Option<ExitStatus>, SidecarError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(SidecarError::Wait)? {
            return Ok(Some(status));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        thread::sleep(remaining.min(EXIT_POLL_INTERVAL));
    }
}

/// Sends `signal`; a process that is already gone counts as success.
fn send_signal(pid: Pid, signal: Signal) -> Result<(), SidecarError> {
    match kill(pid, signal) {
        Ok(()) => Ok(()),
        Err(Errno::ESRCH) => Ok(()),
        Err(source) => Err(SidecarError::Signal { pid, source }),
    }
}

#[cfg(test)]
#[path = "shutdown_test.rs"]
mod shutdown_test;
