//! Whether the CLI process has exited while the app still holds it, read from `/proc`.
//!
//! `Sidecar` keeps its child unreaped until shutdown, so a CLI that died shows as a zombie in
//! `/proc/<pid>/stat`. Shutdown then reaps it and reports the real exit status.

use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use oc_state::ServerExit;

/// What `/proc` says about the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Liveness {
    Running,
    Exited,
}

/// Reads `/proc/<pid>/stat`. A process that is gone or a zombie has exited; a stat file that
/// cannot be read for another reason counts as running, so a read error never ends a session.
pub(crate) fn liveness(pid: i32) -> Liveness {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => liveness_of_stat(&stat),
        Err(error) if error.kind() == ErrorKind::NotFound => Liveness::Exited,
        Err(error) => {
            tracing::warn!(target: crate::logging::SERVER, pid, %error, "[SERVER] Could not read the server's process state");
            Liveness::Running
        }
    }
}

/// The state letter follows the command name, which sits in parentheses and may hold spaces
/// or parentheses itself, so it is read after the last `)`.
pub(crate) fn liveness_of_stat(stat: &str) -> Liveness {
    let state = stat
        .rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().next());
    match state {
        Some("Z" | "X" | "x") => Liveness::Exited,
        Some(_) | None => Liveness::Running,
    }
}

/// The exit status as the header shows it.
pub(crate) fn server_exit(status: ExitStatus) -> ServerExit {
    match (status.code(), status.signal()) {
        (Some(code), _) => ServerExit::Code(code),
        (None, Some(signal)) => ServerExit::Signal(signal),
        (None, None) => ServerExit::Unknown,
    }
}

#[cfg(test)]
#[path = "exit_test.rs"]
mod exit_test;
