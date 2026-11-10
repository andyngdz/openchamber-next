use std::time::Duration;

use super::{CliState, MAX_START_ATTEMPTS, Sidecar};
use crate::errors::SidecarError;
use crate::fake_cli_test::{
    closes_stdout_and_hangs, fake_config, ignores_sigterm, listen_error_in_log, port_in_use,
    prints_port, remove_fake,
};
use crate::shutdown::ShutdownOutcome;

#[test]
fn retries_a_taken_port_then_gives_up() {
    let config = fake_config("sidecar-in-use", &port_in_use());
    let error = Sidecar::start(&config).expect_err("port in use");
    let SidecarError::PortInUse {
        attempts,
        diagnostics,
    } = error
    else {
        unreachable!("unexpected error {error:?}");
    };
    assert_eq!(attempts, MAX_START_ATTEMPTS);
    assert!(crate::port::is_port_in_use_message(
        &diagnostics.stderr_tail
    ));
    remove_fake(&config);
}

#[test]
fn retries_when_the_listen_error_lands_only_in_the_log() {
    let config = fake_config("sidecar-listen-log", &listen_error_in_log());
    let error = Sidecar::start(&config).expect_err("port in use");
    let SidecarError::PortInUse { attempts, .. } = error else {
        unreachable!("unexpected error {error:?}");
    };
    assert_eq!(attempts, MAX_START_ATTEMPTS);
    remove_fake(&config);
}

#[test]
fn a_cli_that_closes_stdout_and_hangs_is_stopped() {
    let mut config = fake_config("sidecar-hang", &closes_stdout_and_hangs());
    config.graceful_timeout = Duration::from_millis(300);
    let error = Sidecar::start(&config).expect_err("exited early");
    let SidecarError::ExitedEarly { diagnostics } = error else {
        unreachable!("unexpected error {error:?}");
    };
    assert!(diagnostics.exit_status.is_some());
    remove_fake(&config);
}

#[test]
fn try_exit_status_follows_the_cli() {
    let config = fake_config("sidecar-state", &prints_port());
    let mut sidecar = Sidecar::start(&config).expect("start");
    assert_eq!(sidecar.try_exit_status().expect("state"), CliState::Running);
    nix::sys::signal::kill(sidecar.cli_pid(), nix::sys::signal::Signal::SIGKILL).expect("kill");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let state = loop {
        let state = sidecar.try_exit_status().expect("state");
        if state != CliState::Running || std::time::Instant::now() > deadline {
            break state;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(matches!(state, CliState::Exited(_)), "{state:?}");
    sidecar.shutdown().expect("shutdown");
    assert_eq!(sidecar.try_exit_status().expect("state"), CliState::Stopped);
    remove_fake(&config);
}

#[test]
fn shutdown_is_graceful_then_idempotent() {
    let config = fake_config("sidecar-graceful", &prints_port());
    let mut sidecar = Sidecar::start(&config).expect("start");
    assert!(sidecar.record_opencode_pids().is_empty());
    let outcome = sidecar.shutdown().expect("shutdown");
    assert!(
        matches!(outcome, ShutdownOutcome::Exited { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        sidecar.shutdown().expect("again"),
        ShutdownOutcome::AlreadyStopped
    );
    remove_fake(&config);
}

#[test]
fn shutdown_kills_a_cli_that_ignores_sigterm() {
    let mut config = fake_config("sidecar-stubborn", &ignores_sigterm());
    config.graceful_timeout = Duration::from_millis(500);
    let mut sidecar = Sidecar::start(&config).expect("start");
    let cli_pid = sidecar.cli_pid();
    let outcome = sidecar.shutdown().expect("shutdown");
    assert!(
        matches!(outcome, ShutdownOutcome::Killed { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        nix::sys::signal::kill(cli_pid, None),
        Err(nix::errno::Errno::ESRCH)
    );
    remove_fake(&config);
}

#[test]
fn drop_stops_the_cli() {
    let config = fake_config("sidecar-drop", &prints_port());
    let sidecar = Sidecar::start(&config).expect("start");
    let cli_pid = sidecar.cli_pid();
    drop(sidecar);
    assert_eq!(
        nix::sys::signal::kill(cli_pid, None),
        Err(nix::errno::Errno::ESRCH)
    );
    remove_fake(&config);
}
