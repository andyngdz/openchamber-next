//! Starts what a live window needs: the server thread, which starts the sidecar while the
//! window opens, and the I/O runtime.

use std::path::{Path, PathBuf};

use oc_state::State;

use super::{LiveWindow, Mode, open};
use crate::errors::AppError;
use crate::live::{IoRuntime, ServerLink};
use crate::server::ServerProcess;

/// Asks the server thread to start the sidecar, starts the I/O runtime, runs the window, and
/// stops both when it closes. The window opens at once on Starting; a sidecar that fails to
/// start shows its failure there.
pub(super) fn run_live(directory: &str) -> Result<(), AppError> {
    let directory = real_directory(Path::new(directory))?;
    let process = ServerProcess::spawn(PathBuf::from(&directory))?;
    let started = process.start();
    let runtime = IoRuntime::start()?;
    let live = LiveWindow {
        link: ServerLink::new(process.clone(), runtime.handle().clone(), directory.clone()),
        process: process.clone(),
        started,
    };
    let outcome = open(State::new(directory), Vec::new(), Mode::Live(live));
    process.stop();
    drop(runtime);
    outcome
}

/// The directory as the server records it: its real path, without symlinks or a trailing
/// slash. The server lists a directory's sessions under the real path and never lists one
/// created under a symlinked path (`sidecar_it`), so the window must ask with the real path.
pub(super) fn real_directory(directory: &Path) -> Result<String, AppError> {
    let real = std::fs::canonicalize(directory).map_err(|source| AppError::ResolveDirectory {
        path: directory.to_path_buf(),
        source,
    })?;
    Ok(real.to_string_lossy().into_owned())
}

#[cfg(test)]
#[path = "live_start_test.rs"]
mod live_start_test;
