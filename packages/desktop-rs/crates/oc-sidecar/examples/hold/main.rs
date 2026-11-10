//! Starts the sidecar, prints `{"cli_pid", "port", "opencode_pids"}` as one
//! JSON line on stdout, then waits forever, so a test can kill this process
//! and check that the parent-death signal stops the server.
//!
//! Settings come from this process's env: `OPENCHAMBER_CLI` or the repository
//! walk for the CLI, the current directory as the working directory, and
//! `HOME`, `OPENCHAMBER_DATA_DIR` and `OPENCODE_CONFIG_CONTENT` inherited by the
//! CLI.

use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use oc_sidecar::{Sidecar, SidecarConfig, SidecarError, locate_cli};

const OPENCODE_WAIT: Duration = Duration::from_secs(15);
const OPENCODE_POLL: Duration = Duration::from_millis(100);

fn main() -> ExitCode {
    let sidecar = match start() {
        Ok(sidecar) => sidecar,
        Err(error) => {
            eprintln!("hold: {error}");
            if let Some(diagnostics) = error.diagnostics() {
                eprintln!("hold: {diagnostics:#?}");
            }
            return ExitCode::FAILURE;
        }
    };
    let line = serde_json::json!({
        "cli_pid": sidecar.cli_pid().as_raw(),
        "port": sidecar.port(),
        "opencode_pids": sidecar.opencode_pids().iter().map(|pid| pid.as_raw()).collect::<Vec<_>>(),
    });
    println!("{line}");
    loop {
        thread::park();
    }
}

/// Starts the sidecar on this thread, the main thread, and waits until the CLI
/// runs OpenCode, which it starts shortly after printing the port.
fn start() -> Result<Sidecar, SidecarError> {
    let working_dir = std::env::current_dir().map_err(|source| SidecarError::Spawn {
        program: "hold".to_owned(),
        source,
    })?;
    let config = SidecarConfig::new(locate_cli()?, working_dir, Vec::new());
    let mut sidecar = Sidecar::start(&config)?;
    let deadline = Instant::now() + OPENCODE_WAIT;
    while sidecar.record_opencode_pids().is_empty() && Instant::now() < deadline {
        thread::sleep(OPENCODE_POLL);
    }
    Ok(sidecar)
}
