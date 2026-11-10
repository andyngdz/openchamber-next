use std::env;
use std::path::{Path, PathBuf};

use crate::errors::SidecarError;

/// Env var that names the CLI entry file directly.
pub const CLI_PATH_ENV: &str = "OPENCHAMBER_CLI";

/// Where the CLI sits below the repository root.
const CLI_RELATIVE_PATH: &str = "packages/web/bin/cli.js";

/// Finds the OpenChamber CLI for this process: `OPENCHAMBER_CLI` when set,
/// else `packages/web/bin/cli.js` under the first ancestor of the executable
/// or of the working directory that has it.
///
/// # Examples
///
/// ```no_run
/// let cli = oc_sidecar::locate_cli().expect("run from the repository");
/// assert!(cli.ends_with("cli.js"));
/// ```
pub fn locate_cli() -> Result<PathBuf, SidecarError> {
    let override_path = env::var_os(CLI_PATH_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let start_dirs: Vec<PathBuf> = [
        env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf)),
        env::current_dir().ok(),
    ]
    .into_iter()
    .flatten()
    .collect();
    resolve_cli_path(override_path, &start_dirs)
}

/// Resolves the CLI path from an explicit override or by walking up from each
/// start directory in order. The override is returned as given, without a
/// file check, so a wrong value fails loudly at spawn time.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// let cli = oc_sidecar::resolve_cli_path(Some(PathBuf::from("/opt/cli.js")), &[]);
/// assert_eq!(cli.expect("override"), PathBuf::from("/opt/cli.js"));
/// ```
pub fn resolve_cli_path(
    override_path: Option<PathBuf>,
    start_dirs: &[PathBuf],
) -> Result<PathBuf, SidecarError> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    start_dirs
        .iter()
        .flat_map(|start| start.ancestors())
        .map(|dir| dir.join(CLI_RELATIVE_PATH))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| SidecarError::CliNotFound {
            searched: start_dirs.to_vec(),
        })
}

#[cfg(test)]
#[path = "cli_path_test.rs"]
mod cli_path_test;
