//! Starts what a live window needs, in the order the parent-death signal requires.

use std::path::{Path, PathBuf};

use oc_state::State;

use super::{LiveWindow, Mode, open};
use crate::errors::AppError;
use crate::live::{IoRuntime, ServerLink};
use crate::server::ServerProcess;

/// Starts the sidecar on this thread, then the I/O runtime, runs the window, and stops both
/// when it closes. A sidecar that fails to start opens the window on the failure.
pub(super) fn run_live(directory: &str) -> Result<(), AppError> {
    let directory = absolute_directory(Path::new(directory))?;
    let process = ServerProcess::new(PathBuf::from(&directory));
    let started = process.start();
    let runtime = IoRuntime::start()?;
    let live = LiveWindow {
        link: ServerLink::new(process.clone(), runtime.handle().clone(), directory.clone()),
        process: process.clone(),
        started,
    };
    let outcome = open(State::new(directory), Vec::new(), Mode::Live(live));
    process.shutdown();
    drop(runtime);
    outcome
}

/// The directory as the server records it: absolute, without a trailing slash.
pub(super) fn absolute_directory(directory: &Path) -> Result<String, AppError> {
    let absolute = std::path::absolute(directory).map_err(|source| AppError::ResolveDirectory {
        path: directory.to_path_buf(),
        source,
    })?;
    let text = absolute.to_string_lossy();
    let trimmed = text.trim_end_matches('/');
    Ok(if trimmed.is_empty() { "/" } else { trimmed }.to_owned())
}

#[cfg(test)]
#[path = "live_start_test.rs"]
mod live_start_test;
