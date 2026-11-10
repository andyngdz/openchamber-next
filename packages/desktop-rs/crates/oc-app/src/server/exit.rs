//! How an exited CLI ended, as the header shows it.

use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use oc_state::ServerExit;

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
