//! Starts what a live window needs: the server thread, which starts the sidecar in the
//! user's home directory while the window opens, and the I/O runtime.

use std::path::{Path, PathBuf};

use oc_client::Directory;
use oc_state::State;

use super::{LiveWindow, Mode, open};
use crate::errors::AppError;
use crate::live::{IoRuntime, ServerLink};
use crate::server::ServerProcess;

/// Asks the server thread to start the sidecar, starts the I/O runtime, runs the window, and
/// stops both when it closes. The sidecar works in the home directory; the window is not
/// opened on one project, so projects come from the shared settings and the draft starts on
/// Chat.
pub(super) fn run_live() -> Result<(), AppError> {
    let home = home_directory()?;
    let process = ServerProcess::spawn(PathBuf::from(home.as_str()))?;
    let started = process.start();
    let runtime = IoRuntime::start()?;
    let link = ServerLink::new(process.clone(), runtime.handle().clone());
    let live = LiveWindow {
        link,
        process: process.clone(),
        started,
    };
    let outcome = open(State::new(home.as_str()), Vec::new(), Mode::Live(live));
    process.stop();
    drop(runtime);
    outcome
}

/// The home directory the sidecar works in: `$HOME`, resolved to its real path.
fn home_directory() -> Result<Directory, AppError> {
    match std::env::var_os("HOME") {
        Some(home) => real_directory(&PathBuf::from(home)),
        None => std::env::current_dir()
            .map_err(|source| AppError::ResolveDirectory {
                path: PathBuf::from("."),
                source,
            })
            .and_then(|current| real_directory(&current)),
    }
}

/// The directory as the server records it: its real path, without symlinks or a trailing
/// slash. The server lists a directory's sessions under the real path and never lists one
/// created under a symlinked path (`sidecar_it`), so the window must ask with the real path.
pub(super) fn real_directory(directory: &Path) -> Result<Directory, AppError> {
    let real = std::fs::canonicalize(directory).map_err(|source| AppError::ResolveDirectory {
        path: directory.to_path_buf(),
        source,
    })?;
    Ok(Directory::new(real.to_string_lossy().into_owned()))
}

#[cfg(test)]
#[path = "live_start_test.rs"]
mod live_start_test;
