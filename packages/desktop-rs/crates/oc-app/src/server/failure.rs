//! A sidecar that failed to start, as the error screen shows it.

use std::error::Error;
use std::fmt::Write;

use oc_sidecar::SidecarError;
use oc_state::ServerFailure;

/// The error and its causes on one line, with the stderr and log tails the start left.
pub(crate) fn start_failure(error: &SidecarError) -> ServerFailure {
    let mut message = format!("Server did not start: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        write!(message, ": {cause}").ok();
        source = cause.source();
    }
    let (stderr_tail, log_tail) = error
        .diagnostics()
        .map(|found| (found.stderr_tail.clone(), found.log_tail.clone()))
        .unwrap_or_default();
    ServerFailure {
        message,
        stderr_tail,
        log_tail,
    }
}

#[cfg(test)]
#[path = "failure_test.rs"]
mod failure_test;
