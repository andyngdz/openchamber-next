//! What the integration cases share: the opt-in switch, an isolated temp root with a git
//! project, seeded settings and a fixture model provider under two provider ids, and waits
//! on processes.

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
use serde_json::{Map, Value, json};

pub const FIXTURE_SCRIPT: &str = "scripts/perf/fixture-provider.mjs";
pub const MODEL: &str = "perf/stream-1200cps";
/// The package the fixture provider is registered with in the native OpenCode config.
const PROVIDER_PACKAGE: &str = "aisdk:@ai-sdk/openai-compatible";
/// The fixture model the pickers' variant cases use, with the Thinking levels below.
const VARIANT_MODEL: &str = "stream-600cps";
/// The model the model-switch cases pick in the second provider.
const SECOND_MODEL: &str = "stream-1200cps";
/// The one primary agent the sessions of the pickers' cases start on.
const FIXTURE_AGENT: &str = "it-primary";
const PROJECT_COLOR: &str = "#4F46E5";
const PROJECT_ICON: &str = "rocket";
/// Lowercase, because the server lowercases a project's `iconBackground` on read.
const PROJECT_ICON_BACKGROUND: &str = "#e0e7ff";
pub const PROVIDER_START_LIMIT: Duration = Duration::from_secs(15);
pub const OPENCODE_START_LIMIT: Duration = Duration::from_secs(30);
pub const GONE_LIMIT: Duration = Duration::from_secs(15);
pub const POLL: Duration = Duration::from_millis(100);

pub static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
pub static CASES: AtomicUsize = AtomicUsize::new(0);

pub fn enabled() -> bool {
    let on = std::env::var("OC_DESKTOP_IT").is_ok_and(|value| value == "1");
    if !on {
        writeln!(std::io::stderr(), "IT: skipped, set OC_DESKTOP_IT=1").ok();
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
        let second_project = root.join("second-project");
        for dir in [
            &project,
            &second_project,
            &root.join("home"),
            &root.join("data"),
        ] {
            fs::create_dir_all(dir).unwrap_or_else(|error| fail("create dir", error));
        }
        let seeded = settings_seed(&project, &second_project);
        fs::write(root.join("data").join("settings.json"), seeded.to_string())
            .unwrap_or_else(|error| fail("seed settings", error));
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

/// The settings document the server starts on. React reads the same keys, so the seed pins
/// the shapes the pickers and the sidebar decode: two projects (the first carrying the
/// fields this app only round-trips, `color`, `icon` and `iconBackground`), the active
/// project, and the pickers' defaults on the fixture catalog.
fn settings_seed(project: &Path, second_project: &Path) -> Value {
    json!({
        "projects": [
            {
                "id": "path-fixture-project",
                "path": project.to_string_lossy(),
                "label": "Fixture project",
                "color": PROJECT_COLOR,
                "icon": PROJECT_ICON,
                "iconBackground": PROJECT_ICON_BACKGROUND,
                "addedAt": 1_800_000_000_000u64,
                "lastOpenedAt": 1_800_000_001_000u64,
                "sidebarCollapsed": false,
            },
            {
                "id": "path-second-project",
                "path": second_project.to_string_lossy(),
                "label": "Second fixture project",
                "addedAt": 1_800_000_002_000u64,
                "lastOpenedAt": 1_800_000_003_000u64,
                "sidebarCollapsed": true,
            },
        ],
        "activeProjectId": "path-fixture-project",
        "defaultModel": format!("perf/{VARIANT_MODEL}"),
        "defaultVariant": "high",
        "defaultAgent": FIXTURE_AGENT,
        "lastSelectedModel": MODEL,
        "favoriteModels": [
            { "providerID": "perf", "modelID": VARIANT_MODEL },
            { "providerID": "perf-alt", "modelID": SECOND_MODEL },
        ],
        "recentModels": [{ "providerID": "perf", "modelID": MODEL }],
        "favoriteAgents": [FIXTURE_AGENT],
        "lastUsedEfforts": { "perf/stream-600cps": "high" },
        "sidebarProjectSortOrder": "a-z",
        "sidebarShowChatsSection": true,
        "sidebarShowWorkSection": true,
        "sidebarShowPinnedSection": true,
    })
}

pub fn opencode_config(port: u16) -> String {
    let output = Command::new("node")
        .arg(repo_root().join(FIXTURE_SCRIPT))
        .arg(port.to_string())
        .arg("--print-config")
        .output()
        .unwrap_or_else(|error| fail("provider config", error));
    let fixture: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| fail("provider config JSON", error));
    let mut models = fixture_models(&fixture);
    set_variants(&mut models, VARIANT_MODEL, &["low", "high"]);
    set_variants(&mut models, SECOND_MODEL, &["low", "medium", "high"]);
    let second_models: Map<String, Value> = [SECOND_MODEL, "stream-300cps"]
        .iter()
        .filter_map(|id| {
            models
                .get(*id)
                .map(|model| ((*id).to_owned(), model.clone()))
        })
        .collect();
    let options = json!({
        "baseURL": format!("http://127.0.0.1:{port}/v1"),
        "apiKey": "unused",
    });
    // The same fixture provider under a second id, so the pickers see two provider
    // sections; the fixture answers any model name, so the id only changes the catalog.
    let mut providers = Map::new();
    for (id, name, catalog) in [
        ("perf", "Performance fixture", models),
        ("perf-alt", "Performance fixture, second id", second_models),
    ] {
        providers.insert(
            id.to_owned(),
            json!({
                "package": PROVIDER_PACKAGE,
                "name": name,
                "settings": options,
                "models": catalog,
            }),
        );
    }
    let mut agents = Map::new();
    agents.insert(
        FIXTURE_AGENT.to_owned(),
        json!({ "description": "Primary fixture agent", "mode": "primary" }),
    );
    json!({
        "providers": providers,
        "agents": agents,
        "model": MODEL,
        "small_model": MODEL,
        "autoupdate": false,
        "share": "disabled",
    })
    .to_string()
}

/// The fixture's models with their variants cleared: the OpenAI-compatible package gives
/// every model the low/medium/high reasoning levels by default, and only
/// [`VARIANT_MODEL`] and [`SECOND_MODEL`] should carry any.
fn fixture_models(fixture: &Value) -> Map<String, Value> {
    let models = fixture["provider"]["perf"]["models"]
        .as_object()
        .unwrap_or_else(|| fail("provider config", "the fixture has no model list"));
    models
        .iter()
        .map(|(id, model)| {
            let mut fields = model.as_object().cloned().unwrap_or_default();
            fields.insert("variants".to_owned(), json!([]));
            (id.clone(), Value::Object(fields))
        })
        .collect()
}

fn set_variants(models: &mut Map<String, Value>, id: &str, variants: &[&str]) {
    let fields = models
        .get_mut(id)
        .and_then(Value::as_object_mut)
        .unwrap_or_else(|| fail("provider config", format!("the fixture has no {id}")));
    let variants = variants.iter().map(|id| json!({ "id": id })).collect();
    fields.insert("variants".to_owned(), Value::Array(variants));
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
