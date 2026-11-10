use std::os::unix::process::ExitStatusExt;
use std::time::Duration;

use super::launch;
use crate::errors::{PortLineError, SidecarError};
use crate::fake_cli_test::{
    crashes_with_log, fake_config, prints_password, prints_port, remove_fake, stays_silent,
};
use crate::port::pick_free_port;
use crate::shutdown::stop_cli;

#[test]
fn reads_the_port_line() {
    let config = fake_config("launch-port", &prints_port());
    let port = pick_free_port().expect("port");
    let mut launched = launch(&config, port).expect("launch");
    assert_eq!(launched.port, port);
    stop_cli(&mut launched.child, &mut Vec::new(), Duration::from_secs(5)).expect("stop");
    remove_fake(&config);
}

#[test]
fn rejects_a_password_line_and_stops_the_cli() {
    let config = fake_config("launch-password", &prints_password());
    let error = launch(&config, pick_free_port().expect("port")).expect_err("password line");
    let SidecarError::InvalidPortLine {
        source,
        diagnostics,
    } = error
    else {
        unreachable!("unexpected error {error:?}");
    };
    assert_eq!(source, PortLineError::PasswordLine);
    let status = diagnostics.exit_status.expect("stopped");
    assert_eq!(status.signal(), Some(libc_sigterm()));
    remove_fake(&config);
}

#[test]
fn early_exit_carries_status_stderr_and_log_tail() {
    let config = fake_config("launch-crash", &crashes_with_log());
    let port = pick_free_port().expect("port");
    let error = launch(&config, port).expect_err("crash");
    let SidecarError::ExitedEarly { diagnostics } = error else {
        unreachable!("unexpected error {error:?}");
    };
    assert_eq!(
        diagnostics.exit_status.and_then(|status| status.code()),
        Some(3)
    );
    assert_eq!(diagnostics.stderr_tail, vec!["Error: boom".to_owned()]);
    assert_eq!(diagnostics.log_tail, vec!["boom in log".to_owned()]);
    assert_eq!(
        diagnostics.log_path,
        Some(
            config
                .working_dir
                .join(format!("logs/openchamber-{port}.log"))
        )
    );
    remove_fake(&config);
}

#[test]
fn times_out_without_a_line() {
    let mut config = fake_config("launch-silent", &stays_silent());
    config.first_line_timeout = Duration::from_millis(500);
    let error = launch(&config, pick_free_port().expect("port")).expect_err("timeout");
    let SidecarError::Timeout {
        waited,
        diagnostics,
    } = error
    else {
        unreachable!("unexpected error {error:?}");
    };
    assert_eq!(waited, Duration::from_millis(500));
    assert!(diagnostics.exit_status.is_some());
    remove_fake(&config);
}

#[test]
fn missing_program_is_a_spawn_error() {
    let mut config = fake_config("launch-missing-dir", &stays_silent());
    config.working_dir = config.working_dir.join("does-not-exist");
    let error = launch(&config, pick_free_port().expect("port")).expect_err("spawn");
    assert!(matches!(error, SidecarError::Spawn { .. }), "{error:?}");
}

fn libc_sigterm() -> i32 {
    nix::sys::signal::Signal::SIGTERM as i32
}
