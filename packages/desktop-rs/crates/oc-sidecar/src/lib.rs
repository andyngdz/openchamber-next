//! Spawns the OpenChamber server as a child process and stops it again.
//!
//! The sidecar runs `node <cli> serve --foreground --quiet --api-only` on a
//! free port, reads the port from the first stdout line, and on shutdown stops
//! the CLI and the `opencode serve` processes it manages. Linux only.

mod cli_path;
mod config;
mod errors;
#[cfg(test)]
mod fake_cli_test;
mod launch;
mod log_file;
mod logging;
mod pipes;
mod port;
mod proc_scan;
mod shutdown;
mod sidecar;

pub use cli_path::{CLI_PATH_ENV, locate_cli, resolve_cli_path};
pub use config::SidecarConfig;
pub use errors::{PortLineError, SidecarError, StartupDiagnostics};
pub use log_file::{TAIL_LINES, log_file_path, resolve_data_dir};
pub use nix::unistd::Pid;
pub use port::{is_port_in_use_message, parse_port_line, pick_free_port};
pub use shutdown::ShutdownOutcome;
pub use sidecar::{CliState, MAX_START_ATTEMPTS, Sidecar};
