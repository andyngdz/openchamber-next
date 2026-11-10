use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::RecvTimeoutError;

use nix::errno::Errno;
use nix::sys::prctl;
use nix::sys::signal::Signal;
use nix::unistd::{Pid, getpid, getppid};

use crate::config::SidecarConfig;
use crate::errors::{PortLineError, SidecarError, StartupDiagnostics};
use crate::log_file::{LogCursor, log_file_path};
use crate::pipes::{FirstLine, StderrTail, follow_stdout};
use crate::port::parse_port_line;
use crate::shutdown::{ShutdownOutcome, stop_cli, wait_for_exit};

const NODE_PROGRAM: &str = "node";

/// A CLI that printed a valid port line.
#[derive(Debug)]
pub(crate) struct Launched {
    pub(crate) child: Child,
    pub(crate) port: u16,
}

/// Why one start attempt did not produce a port.
enum StartFailure {
    Exited,
    InvalidLine(PortLineError),
    ReadFailed(io::Error),
    TimedOut,
}

/// Runs one start attempt on `port` and waits for the port line. On failure
/// the CLI is stopped, and the error carries its stderr and log tails.
pub(crate) fn launch(config: &SidecarConfig, port: u16) -> Result<Launched, SidecarError> {
    let log = config
        .data_dir()
        .map(|dir| LogCursor::at_end_of(log_file_path(&dir, port)));
    let mut child = spawn_cli(config, port)?;
    let mut stderr = StderrTail::follow(child.stderr.take())?;
    let first_line = follow_stdout(child.stdout.take())?;
    let failure = match first_line.recv_timeout(config.first_line_timeout) {
        Ok(FirstLine::Line(line)) => match parse_port_line(&line) {
            Ok(printed) => {
                return Ok(Launched {
                    child,
                    port: printed,
                });
            }
            Err(source) => StartFailure::InvalidLine(source),
        },
        Ok(FirstLine::Closed) => StartFailure::Exited,
        Err(RecvTimeoutError::Disconnected) => StartFailure::Exited,
        Ok(FirstLine::Failed(source)) => StartFailure::ReadFailed(source),
        Err(RecvTimeoutError::Timeout) => StartFailure::TimedOut,
    };
    let exit_status = match failure {
        StartFailure::Exited => {
            // Stdout closed, so the CLI is exiting; one that hangs after
            // closing it gets the stop path instead of blocking the caller.
            let status = match wait_for_exit(&mut child, config.graceful_timeout)? {
                Some(status) => Some(status),
                None => {
                    let outcome = stop_cli(&mut child, &mut Vec::new(), config.graceful_timeout)?;
                    outcome_status(&outcome)
                }
            };
            stderr.wait_for_close();
            status
        }
        StartFailure::InvalidLine(_) | StartFailure::ReadFailed(_) | StartFailure::TimedOut => {
            let outcome = stop_cli(&mut child, &mut Vec::new(), config.graceful_timeout)?;
            outcome_status(&outcome)
        }
    };
    let diagnostics = StartupDiagnostics {
        exit_status,
        stderr_tail: stderr.snapshot(),
        log_path: log.as_ref().map(|cursor| cursor.path().to_path_buf()),
        log_tail: log.as_ref().map(LogCursor::tail).unwrap_or_default(),
    };
    Err(failure.into_error(config, diagnostics))
}

impl StartFailure {
    fn into_error(self, config: &SidecarConfig, diagnostics: StartupDiagnostics) -> SidecarError {
        match self {
            Self::Exited => SidecarError::ExitedEarly { diagnostics },
            Self::InvalidLine(source) => SidecarError::InvalidPortLine {
                source,
                diagnostics,
            },
            Self::ReadFailed(source) => SidecarError::ReadStdout(source),
            Self::TimedOut => SidecarError::Timeout {
                waited: config.first_line_timeout,
                diagnostics,
            },
        }
    }
}

fn outcome_status(outcome: &ShutdownOutcome) -> Option<ExitStatus> {
    match outcome {
        ShutdownOutcome::AlreadyStopped => None,
        ShutdownOutcome::Exited { status, .. } | ShutdownOutcome::Killed { status, .. } => {
            Some(*status)
        }
    }
}

fn spawn_cli(config: &SidecarConfig, port: u16) -> Result<Child, SidecarError> {
    let mut command = Command::new(NODE_PROGRAM);
    command
        .arg(&config.cli_path)
        .args(["serve", "--foreground", "--quiet", "--api-only"])
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        .current_dir(&config.working_dir)
        .envs(config.extra_env.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    set_parent_death_signal(&mut command);
    command.spawn().map_err(|source| SidecarError::Spawn {
        program: NODE_PROGRAM.to_owned(),
        source,
    })
}

/// Makes the kernel send SIGTERM to the child when the spawning thread exits.
fn set_parent_death_signal(command: &mut Command) {
    let parent: Pid = getpid();
    // Rule Ignore: RUST011
    // Reason: pre_exec is the only std hook that runs in the forked child before exec, and PR_SET_PDEATHSIG must be set there.
    // SAFETY: the closure runs between fork and exec and only makes the
    // prctl and getppid syscalls; it allocates nothing and takes no lock, so
    // it is async-signal-safe.
    unsafe {
        command.pre_exec(move || {
            prctl::set_pdeathsig(Signal::SIGTERM).map_err(io::Error::from)?;
            // The parent may have died between fork and prctl; then no signal
            // would ever come, so the child must not start.
            if getppid() != parent {
                return Err(io::Error::from(Errno::ESRCH));
            }
            Ok(())
        });
    }
}

#[cfg(test)]
#[path = "launch_test.rs"]
mod launch_test;
