//! The sidecar against a real OpenChamber server and OpenCode, with a fixture model provider.
//!
//! The cases start `node packages/web/bin/cli.js`, so they run only with `OC_DESKTOP_IT=1`:
//! `OC_DESKTOP_IT=1 cargo test -p oc-app --test sidecar_it -- --nocapture`.
//! Each case has its own isolated temp root and fixture provider, removed by a guard; the
//! cases run one at a time.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use futures::StreamExt;
use oc_client::{Client, EventData};
use oc_sidecar::{ShutdownOutcome, Sidecar, SidecarConfig, locate_cli};
use serde_json::Value;

mod common;

use common::{Env, enabled, fail, is_gone, wait_for_opencode, wait_until_gone};

const DELTA_LIMIT: Duration = Duration::from_secs(60);
const HOLD_LINE_LIMIT: Duration = Duration::from_secs(60);

/// Auth, list, create, open events, send a prompt, wait for that session's text delta.
async fn drive_session(client: &Client) {
    client
        .check_auth()
        .await
        .unwrap_or_else(|error| fail("auth", error));
    client
        .list_sessions()
        .await
        .unwrap_or_else(|error| fail("list sessions", error));
    let session = client
        .create_session()
        .await
        .unwrap_or_else(|error| fail("create session", error));
    let mut events = client
        .events()
        .await
        .unwrap_or_else(|error| fail("open events", error));
    client
        .send_prompt(&session.id, "Say hello.")
        .await
        .unwrap_or_else(|error| fail("send prompt", error));
    let wait = async {
        while let Some(item) = events.next().await {
            if let Ok(event) = item
                && let EventData::TextDelta(delta) = event.data
                && delta.session_id == session.id
            {
                return;
            }
        }
        fail("event stream", "ended before a text delta arrived");
    };
    if tokio::time::timeout(DELTA_LIMIT, wait).await.is_err() {
        fail("text delta", format!("none within {DELTA_LIMIT:?}"));
    }
}

#[test]
fn start_prompt_stream_and_shut_down() {
    if !enabled() {
        return;
    }
    let started = Instant::now();
    let env = Env::new();
    let mut sidecar = Sidecar::start(&env.sidecar_config(SidecarConfig::DEFAULT_GRACEFUL_TIMEOUT))
        .unwrap_or_else(|error| fail("start sidecar", error));
    let cli_pid = sidecar.cli_pid().as_raw();
    let opencode = wait_for_opencode(&mut sidecar);
    let directory = env.project.to_string_lossy().into_owned();
    let client =
        Client::new(sidecar.port(), directory).unwrap_or_else(|error| fail("client", error));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| fail("tokio runtime", error));
    runtime.block_on(drive_session(&client));

    let outcome = sidecar
        .shutdown()
        .unwrap_or_else(|error| fail("shutdown", error));
    assert!(
        matches!(outcome, ShutdownOutcome::Exited { .. }),
        "expected a graceful exit, got {outcome:?}"
    );
    wait_until_gone("cli", &[cli_pid]);
    wait_until_gone("opencode", &opencode);
    writeln!(
        std::io::stderr(),
        "sidecar_it: full flow took {:?}",
        started.elapsed()
    )
    .ok();
}

#[test]
fn stopped_cli_is_killed_after_the_grace_period() {
    if !enabled() {
        return;
    }
    let env = Env::new();
    let mut sidecar = Sidecar::start(&env.sidecar_config(Duration::from_secs(2)))
        .unwrap_or_else(|error| fail("start sidecar", error));
    let cli_pid = sidecar.cli_pid();
    let opencode = wait_for_opencode(&mut sidecar);
    let stopped = Command::new("kill")
        .arg("-STOP")
        .arg(cli_pid.to_string())
        .status()
        .unwrap_or_else(|error| fail("kill -STOP", error));
    assert!(stopped.success(), "SIGSTOP to the CLI failed");

    let outcome = sidecar
        .shutdown()
        .unwrap_or_else(|error| fail("shutdown", error));
    assert!(
        matches!(outcome, ShutdownOutcome::Killed { .. }),
        "expected the SIGKILL path, got {outcome:?}"
    );
    wait_until_gone("cli", &[cli_pid.as_raw()]);
    wait_until_gone("opencode", &opencode);
}

/// Builds the `hold` example, so it is current, and returns where cargo put it.
fn build_hold() -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "-p", "oc-sidecar", "--example", "hold"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| fail("cargo build hold", error));
    assert!(status.success(), "building the hold example failed");
    let exe = std::env::current_exe().unwrap_or_else(|error| fail("current exe", error));
    let profile_dir = exe
        .ancestors()
        .nth(2)
        .unwrap_or_else(|| fail("target dir", "the test binary is not under deps/"));
    let hold = profile_dir.join("examples").join("hold");
    assert!(hold.is_file(), "no hold example at {}", hold.display());
    hold
}

struct HoldGuard(Child);

impl Drop for HoldGuard {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn first_line(child: &mut Child, limit: Duration) -> String {
    let Some(stdout) = child.stdout.take() else {
        fail("hold stdout", "not piped");
    };
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).ok();
        sender.send(line).ok();
    });
    let line = receiver
        .recv_timeout(limit)
        .unwrap_or_else(|_| fail("hold line", format!("none within {limit:?}")));
    reader.join().ok();
    line
}

fn pid_of(value: &Value) -> Option<i32> {
    value.as_i64().and_then(|pid| i32::try_from(pid).ok())
}

#[test]
fn killed_parent_takes_the_server_down() {
    if !enabled() {
        return;
    }
    let hold = build_hold();
    let env = Env::new();
    let cli = locate_cli().unwrap_or_else(|error| fail("locate cli", error));
    let mut command = Command::new(hold);
    command
        .current_dir(&env.project)
        .env("OPENCHAMBER_CLI", cli)
        .envs(env.child_env())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut guard = HoldGuard(
        command
            .spawn()
            .unwrap_or_else(|error| fail("start hold", error)),
    );
    let line = first_line(&mut guard.0, HOLD_LINE_LIMIT);
    let info: Value = serde_json::from_str(line.trim())
        .unwrap_or_else(|error| fail("hold line JSON", format!("{error}: {line:?}")));
    let cli_pid = pid_of(&info["cli_pid"]).unwrap_or_else(|| fail("hold line", "no cli_pid"));
    let opencode: Vec<i32> = info["opencode_pids"]
        .as_array()
        .map(|items| items.iter().filter_map(pid_of).collect())
        .unwrap_or_default();
    assert!(
        !opencode.is_empty(),
        "hold printed no opencode pids: {line}"
    );
    assert!(!is_gone(cli_pid), "the CLI was not running before the kill");

    guard
        .0
        .kill()
        .unwrap_or_else(|error| fail("SIGKILL hold", error));
    guard.0.wait().ok();
    wait_until_gone("cli", &[cli_pid]);
    wait_until_gone("opencode", &opencode);
}

/// The server lists a directory's sessions under its real path, whatever path the list asks
/// for, and a session created under a symlinked path never shows in any list. The app
/// therefore canonicalizes its directory before it creates or lists anything.
#[test]
fn sessions_are_listed_under_the_real_path_of_a_symlinked_project() {
    if !enabled() {
        return;
    }
    let env = Env::new();
    let link = env.root.join("link");
    std::os::unix::fs::symlink(&env.project, &link).unwrap_or_else(|error| fail("symlink", error));
    let mut sidecar = Sidecar::start(&env.sidecar_config(SidecarConfig::DEFAULT_GRACEFUL_TIMEOUT))
        .unwrap_or_else(|error| fail("start sidecar", error));
    wait_for_opencode(&mut sidecar);
    let client = |directory: &Path| {
        Client::new(sidecar.port(), directory.to_string_lossy().into_owned())
            .unwrap_or_else(|error| fail("client", error))
    };
    let (through_link, real) = (client(&link), client(&env.project));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| fail("tokio runtime", error));
    let project = env.project.to_string_lossy().into_owned();

    let (lost, kept, listed) = runtime.block_on(async {
        real.check_auth()
            .await
            .unwrap_or_else(|error| fail("auth", error));
        let lost = through_link
            .create_session()
            .await
            .unwrap_or_else(|error| fail("create through the link", error));
        let kept = real
            .create_session()
            .await
            .unwrap_or_else(|error| fail("create", error));
        let listed = through_link
            .list_sessions()
            .await
            .unwrap_or_else(|error| fail("list through the link", error));
        (lost, kept, listed)
    });
    sidecar.shutdown().ok();

    let listed: Vec<(&str, &str)> = listed
        .iter()
        .map(|session| (session.id.as_str(), session.location.directory.as_str()))
        .collect();
    assert_eq!(listed, [(kept.id.as_str(), project.as_str())]);
    assert!(
        !listed.iter().any(|(id, _)| *id == lost.id),
        "a session created under the link path is listed: {listed:?}"
    );
}
