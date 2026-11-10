//! What the pickers' integration cases share: a sidecar on the env's project, its shutdown,
//! and the single-threaded runtime the client calls run on.

use oc_sidecar::{Sidecar, SidecarConfig};

use crate::common::{Env, fail, wait_for_opencode, wait_until_gone};

/// A sidecar on the env's project, stopped when it drops.
pub fn start(env: &Env) -> Sidecar {
    let mut sidecar = Sidecar::start(&env.sidecar_config(SidecarConfig::DEFAULT_GRACEFUL_TIMEOUT))
        .unwrap_or_else(|error| fail("start sidecar", error));
    wait_for_opencode(&mut sidecar);
    sidecar
}

/// Stops the sidecar and waits for every process it started to be gone.
pub fn stop(mut sidecar: Sidecar) {
    let cli = sidecar.cli_pid().as_raw();
    let opencode: Vec<i32> = sidecar
        .opencode_pids()
        .iter()
        .map(|pid| pid.as_raw())
        .collect();
    sidecar
        .shutdown()
        .unwrap_or_else(|error| fail("shutdown", error));
    wait_until_gone("cli", &[cli]);
    wait_until_gone("opencode", &opencode);
}

/// The current-thread runtime the client calls run on.
pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| fail("tokio runtime", error))
}
