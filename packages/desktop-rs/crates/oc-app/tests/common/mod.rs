//! What the sidecar integration cases share: the opt-in switch, an isolated temp root with
//! a git project and a fixture model provider, and waits on processes.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use oc_sidecar::{Sidecar, SidecarConfig, locate_cli, pick_free_port};
use serde_json::{Value, json};

pub const FIXTURE_SCRIPT: &str = "scripts/perf/fixture-provider.mjs";
pub const MODEL: &str = "perf/stream-1200cps";
pub const PROVIDER_START_LIMIT: Duration = Duration::from_secs(15);
pub const OPENCODE_START_LIMIT: Duration = Duration::from_secs(30);
pub const GONE_LIMIT: Duration = Duration::from_secs(15);
pub const POLL: Duration = Duration::from_millis(100);

pub static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
pub static CASES: AtomicUsize = AtomicUsize::new(0);

pub fn enabled() -> bool {
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

pub fn fail(what: &str, error: impl std::fmt::Display) -> ! {
    panic!("{what}: {error}")
}

/// The nearest ancestor of the fixtures directory that has the provider script.
pub fn repo_root() -> PathBuf {
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

pub struct Env {
    pub root: PathBuf,
    pub project: PathBuf,
    pub provider: Child,
    pub config: String,
    _serial: MutexGuard<'static, ()>,
}

impl Env {
    pub fn new() -> Self {
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
    pub fn child_env(&self) -> Vec<(OsString, OsString)> {
        vec![
            ("HOME".into(), self.root.join("home").into()),
            ("OPENCHAMBER_DATA_DIR".into(), self.root.join("data").into()),
            ("OPENCODE_CONFIG_CONTENT".into(), self.config.clone().into()),
        ]
    }

    pub fn sidecar_config(&self, graceful: Duration) -> SidecarConfig {
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

pub fn start_provider(port: u16) -> Child {
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

pub fn opencode_config(port: u16) -> String {
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

pub fn is_gone(pid: i32) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(_) => true,
        Ok(stat) => stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next().map(|state| state == "Z"))
            .unwrap_or(true),
    }
}

pub fn wait_until_gone(label: &str, pids: &[i32]) {
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

pub fn wait_for_opencode(sidecar: &mut Sidecar) -> Vec<i32> {
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
