use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use nix::sys::signal::Signal;

use super::{ShutdownOutcome, stop_cli};

/// Starts `sh` running `script` and returns once it printed its first line,
/// so the script's `trap` is in place before the test signals it.
fn spawn_ready(script: &str) -> Child {
    let mut child = Command::new("sh")
        .args(["-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn sh");
    let stdout = child.stdout.take().expect("stdout pipe");
    let mut ready = String::new();
    BufReader::new(stdout)
        .read_line(&mut ready)
        .expect("ready line");
    child
}

#[test]
fn sigterm_stops_a_cooperative_process() {
    let mut child = spawn_ready("echo ready; exec sleep 30");
    let outcome = stop_cli(&mut child, &mut Vec::new(), Duration::from_secs(5)).expect("stop");
    let ShutdownOutcome::Exited { status, .. } = outcome else {
        unreachable!("unexpected outcome {outcome:?}");
    };
    assert_eq!(status.signal(), Some(Signal::SIGTERM as i32));
}

#[test]
fn sigkill_follows_when_sigterm_is_ignored() {
    let mut child = spawn_ready("trap '' TERM; echo ready; exec sleep 30");
    let outcome = stop_cli(&mut child, &mut Vec::new(), Duration::from_millis(300)).expect("stop");
    let ShutdownOutcome::Killed {
        status,
        terminated_opencode,
    } = outcome
    else {
        unreachable!("unexpected outcome {outcome:?}");
    };
    assert_eq!(status.signal(), Some(Signal::SIGKILL as i32));
    assert!(terminated_opencode.is_empty());
}

#[test]
fn an_exited_process_reports_its_status() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    child.wait().expect("wait");
    let outcome = stop_cli(&mut child, &mut Vec::new(), Duration::from_secs(1)).expect("stop");
    let ShutdownOutcome::Exited { status, .. } = outcome else {
        unreachable!("unexpected outcome {outcome:?}");
    };
    assert!(status.success());
}
