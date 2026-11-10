use oc_sidecar::{SidecarError, StartupDiagnostics};

use super::start_failure;

#[test]
fn a_spawn_error_names_the_program_and_the_os_error() {
    let error = SidecarError::Spawn {
        program: "node".to_owned(),
        source: std::io::Error::from(std::io::ErrorKind::NotFound),
    };

    let failure = start_failure(&error);

    assert_eq!(
        failure.message,
        "Server did not start: could not start node: entity not found"
    );
    assert!(failure.stderr_tail.is_empty() && failure.log_tail.is_empty());
}

#[test]
fn an_early_exit_carries_both_tails() {
    let error = SidecarError::ExitedEarly {
        diagnostics: StartupDiagnostics {
            stderr_tail: vec!["boom".to_owned()],
            log_tail: vec!["log line".to_owned()],
            ..StartupDiagnostics::default()
        },
    };

    let failure = start_failure(&error);

    assert_eq!(
        failure.message,
        "Server did not start: the server exited before printing its port"
    );
    assert_eq!(failure.stderr_tail, ["boom"]);
    assert_eq!(failure.log_tail, ["log line"]);
}
