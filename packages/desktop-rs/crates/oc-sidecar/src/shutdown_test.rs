use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::{ShutdownOutcome, stop_cli, terminate_survivors};
use crate::proc_scan::RecordedPid;

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

/// A process whose command line reads `opencode serve`, blocked on stdin.
/// Returns once the shell printed its ready line: `spawn` returns while the
/// kernel still swaps in the new image, when `/proc/<pid>/cmdline` is empty.
fn spawn_fake_opencode() -> Child {
    let mut child = Command::new("sh")
        .args(["-c", "echo ready; read line", "opencode", "serve"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn fake opencode");
    let stdout = child.stdout.take().expect("stdout pipe");
    let mut ready = String::new();
    BufReader::new(stdout)
        .read_line(&mut ready)
        .expect("ready line");
    child
}

#[test]
fn a_recorded_process_is_terminated_while_its_start_time_matches() {
    let mut fake = spawn_fake_opencode();
    let recorded = RecordedPid::capture(Pid::from_raw(fake.id().cast_signed())).expect("record");
    let terminated = terminate_survivors(&[recorded]).expect("terminate");
    assert_eq!(terminated, vec![recorded.pid]);
    let status = fake.wait().expect("wait");
    assert_eq!(status.signal(), Some(Signal::SIGTERM as i32));
}

#[test]
fn a_reused_pid_with_another_start_time_is_left_alone() {
    let mut fake = spawn_fake_opencode();
    let real = RecordedPid::capture(Pid::from_raw(fake.id().cast_signed())).expect("record");
    let stale = RecordedPid {
        start_time: real.start_time + 1,
        ..real
    };
    assert!(terminate_survivors(&[stale]).expect("terminate").is_empty());
    assert!(fake.try_wait().expect("try_wait").is_none());
    fake.kill().expect("kill");
    fake.wait().expect("wait");
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
