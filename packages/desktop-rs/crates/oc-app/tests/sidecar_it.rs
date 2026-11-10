//! The sidecar against a real OpenChamber server and OpenCode, with a fixture model provider.
//!
//! The cases start `node packages/web/bin/cli.js`, so they run only with `OC_DESKTOP_IT=1`:
//! `OC_DESKTOP_IT=1 cargo test -p oc-app --test sidecar_it -- --nocapture`.
//! Each case has its own isolated temp root and fixture provider, removed by a guard; the
//! cases run one at a time.

use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use futures::StreamExt;
use oc_client::{Client, EventData};
use oc_sidecar::{ShutdownOutcome, Sidecar, SidecarConfig, locate_cli, pick_free_port};
use serde_json::{Value, json};

const FIXTURE_SCRIPT: &str = "scripts/perf/fixture-provider.mjs";
const MODEL: &str = "perf/stream-1200cps";
const PROVIDER_START_LIMIT: Duration = Duration::from_secs(15);
const OPENCODE_START_LIMIT: Duration = Duration::from_secs(30);
const DELTA_LIMIT: Duration = Duration::from_secs(60);
const GONE_LIMIT: Duration = Duration::from_secs(15);
const HOLD_LINE_LIMIT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(100);

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
static CASES: AtomicUsize = AtomicUsize::new(0);

fn enabled() -> bool {
    let on = std::env::var("OC_DESKTOP_IT").is_ok_and(|value| value == "1");
    if !on {
        writeln!(
            std::io::stderr(),
            "sidecar_it: skipped, set OC_DESKTOP_IT=1"
        )
        .ok();
    }
    on
}

fn fail(what: &str, error: impl std::fmt::Display) -> ! {
    panic!("{what}: {error}")
}

/// The nearest ancestor of the fixtures directory that has the provider script.
fn repo_root() -> PathBuf {
    Path::new(env!("OC_DESKTOP_FIXTURES"))
        .ancestors()
        .find(|dir| dir.join(FIXTURE_SCRIPT).is_file())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            fail(
                "repo root",
                "no ancestor of the fixtures dir has the provider",
            )
        })
}

struct Env {
    root: PathBuf,
    project: PathBuf,
    provider: Child,
    config: String,
    _serial: MutexGuard<'static, ()>,
}

impl Env {
    fn new() -> Self {
        let serial = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let index = CASES.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("oc-app-sidecar-it-{}-{index}", std::process::id()));
        let project = root.join("project");
        for dir in [&project, &root.join("home"), &root.join("data")] {
            fs::create_dir_all(dir).unwrap_or_else(|error| fail("create dir", error));
        }
        let port = pick_free_port().unwrap_or_else(|error| fail("free port", error));
        let provider = start_provider(port);
        let config = opencode_config(port);
        let env = Self {
            root,
            project,
            provider,
            config,
            _serial: serial,
        };
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&env.project)
            .status()
            .unwrap_or_else(|error| fail("git init", error));
        assert!(status.success(), "git init failed in the scratch project");
        env
    }

    /// The env the CLI sees: isolated home, data dir and OpenCode config.
    fn child_env(&self) -> Vec<(OsString, OsString)> {
        vec![
            ("HOME".into(), self.root.join("home").into()),
            ("OPENCHAMBER_DATA_DIR".into(), self.root.join("data").into()),
            ("OPENCODE_CONFIG_CONTENT".into(), self.config.clone().into()),
        ]
    }

    fn sidecar_config(&self, graceful: Duration) -> SidecarConfig {
        let cli = locate_cli().unwrap_or_else(|error| fail("locate cli", error));
        let mut config = SidecarConfig::new(cli, self.project.clone(), self.child_env());
        config.graceful_timeout = graceful;
        config
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        self.provider.kill().ok();
        self.provider.wait().ok();
        fs::remove_dir_all(&self.root).ok();
    }
}

fn start_provider(port: u16) -> Child {
    let mut child = Command::new("node")
        .arg(repo_root().join(FIXTURE_SCRIPT))
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| fail("start fixture provider", error));
    let deadline = Instant::now() + PROVIDER_START_LIMIT;
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        if Instant::now() >= deadline {
            child.kill().ok();
            child.wait().ok();
            fail("fixture provider", "did not listen in time");
        }
        thread::sleep(POLL);
    }
    child
}

fn opencode_config(port: u16) -> String {
    let output = Command::new("node")
        .arg(repo_root().join(FIXTURE_SCRIPT))
        .arg(port.to_string())
        .arg("--print-config")
        .output()
        .unwrap_or_else(|error| fail("provider config", error));
    let mut config: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| fail("provider config JSON", error));
    config["model"] = json!(MODEL);
    config["small_model"] = json!(MODEL);
    config["autoupdate"] = json!(false);
    config["share"] = json!("disabled");
    config.to_string()
}

fn is_gone(pid: i32) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(_) => true,
        Ok(stat) => stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next().map(|state| state == "Z"))
            .unwrap_or(true),
    }
}

fn wait_until_gone(label: &str, pids: &[i32]) {
    let deadline = Instant::now() + GONE_LIMIT;
    loop {
        let alive: Vec<i32> = pids.iter().copied().filter(|pid| !is_gone(*pid)).collect();
        if alive.is_empty() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{label}: pids still running after {GONE_LIMIT:?}: {alive:?}"
        );
        thread::sleep(POLL);
    }
}

fn wait_for_opencode(sidecar: &mut Sidecar) -> Vec<i32> {
    let deadline = Instant::now() + OPENCODE_START_LIMIT;
    while sidecar.record_opencode_pids().is_empty() {
        assert!(
            Instant::now() < deadline,
            "no opencode serve child within {OPENCODE_START_LIMIT:?}"
        );
        thread::sleep(POLL);
    }
    sidecar
        .opencode_pids()
        .iter()
        .map(|pid| pid.as_raw())
        .collect()
}

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
