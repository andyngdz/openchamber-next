//! Layout parity with the Electron desktop (`fixtures/reference/`), and a check that the
//! binary carries no build-machine path.
//!
//! `--dump-layout` renders in a headless window, so these cases run without a display, in
//! `cargo test --workspace`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

mod parity_support;

use parity_support::{REGIONS, control_differences};

fn fixtures() -> &'static Path {
    Path::new(env!("OC_DESKTOP_FIXTURES"))
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not JSON: {error}", path.display()))
}

fn reference(state: &str) -> Value {
    read_json(&fixtures().join("reference").join(format!("{state}.json")))
}

/// Longest a dump may take; a headless renderer that never produces a frame fails here
/// instead of hanging the suite.
const DUMP_LIMIT: Duration = Duration::from_secs(60);

/// Runs `oc-app --dump-layout` for one state fixture and returns the dump.
///
/// The child renders in its own headless window, so the cases run in parallel and need no
/// display or compositor.
fn dump(state: &str) -> Value {
    static DUMPS: AtomicUsize = AtomicUsize::new(0);
    let run_index = DUMPS.fetch_add(1, Ordering::Relaxed);
    let output = std::env::temp_dir().join(format!(
        "oc-app-parity-{}-{run_index}-{state}.json",
        std::process::id()
    ));
    let fixture = fixtures().join("states").join(format!("{state}.json"));
    let child = Command::new(env!("CARGO_BIN_EXE_oc-app"))
        .arg("--dump-layout")
        .arg(&output)
        .arg("--state")
        .arg(&fixture)
        .stderr(Stdio::piped())
        .spawn()
        .expect("oc-app starts");
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = std::thread::spawn(move || sender.send(child.wait_with_output()));
    let Ok(run) = receiver.recv_timeout(DUMP_LIMIT) else {
        Command::new("kill").arg(pid.to_string()).status().ok();
        waiter.join().ok();
        panic!("oc-app --dump-layout {state} drew no frame within {DUMP_LIMIT:?}");
    };
    // The waiter only sends; the answer was already received above.
    waiter.join().expect("waiter thread ends").ok();
    let run = run.expect("oc-app runs");
    assert!(
        run.status.success(),
        "oc-app --dump-layout {state} failed: {}\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    let layout = read_json(&output);
    std::fs::remove_file(&output).ok();
    layout
}

/// Keys of controls inside markdown content, which the spike does not render, that the
/// reference lists in `state`. An entry skips only the states its `states` field names.
fn content_controls(state: &str) -> HashSet<(String, String)> {
    let list = read_json(&fixtures().join("reference").join("content-controls.json"));
    list["controls"]
        .as_array()
        .expect("content-controls.json has a controls array")
        .iter()
        .filter(|entry| {
            entry["states"]
                .as_array()
                .expect("every content control lists its states")
                .iter()
                .any(|listed| listed.as_str() == Some(state))
        })
        .map(|entry| {
            let field = |name: &str| entry[name].as_str().unwrap_or_default().to_owned();
            (field("region"), field("key"))
        })
        .collect()
}

/// The regions whose reference bounds are not on any device-pixel grid: React centers the
/// composer in the chat column (x 315.75, width 720.5) and the empty state places it at
/// y 387.6. GPUI snaps every edge to the device grid, so it is compared on that grid. The
/// open popup has the same property: Base UI places it at CSS offsets such as x 318/y 386
/// (DraftTargetSelectors.tsx:363-370), and the headless window snaps those to 317.5/385.5 on
/// its 2x grid. Decision trace, "Popover bounds".
const SNAPPED_REGIONS: [&str; 1] = ["composer"];

fn bounds(layout: &Value, region: &str) -> [f64; 4] {
    let side = |name: &str| {
        layout["regions"][region]["bounds"][name]
            .as_f64()
            .unwrap_or(f64::NAN)
    };
    [side("x"), side("y"), side("width"), side("height")]
}

/// Left, top, right and bottom edge of `[x, y, width, height]`.
fn edges([x, y, width, height]: [f64; 4]) -> [f64; 4] {
    [x, y, x + width, y + height]
}

/// Whether every edge lands on the same device pixel, give or take one.
///
/// GPUI rounds each edge to the nearest device pixel (`round(edge * scale)`), and the
/// reference edge is an exact CSS value. Rounding the reference edge the same way puts both
/// on the grid; two values within half a device pixel of each other can still round to
/// adjacent grid points, so equal or one device pixel apart is the closest a snapped layout
/// can come. Anything further is a real placement difference.
fn on_device_grid(expected: [f64; 4], actual: [f64; 4], scale: f64) -> bool {
    edges(expected)
        .iter()
        .zip(edges(actual))
        .all(|(want, got)| ((want * scale).round() - (got * scale).round()).abs() <= 1.)
}

/// Every region whose bounds differ, with both values. Regions match exactly, except the
/// composer, which matches on the device grid of the scale the dump ran at, and the popover,
/// whose placement follows the gpui-kit components rather than Base UI.
fn region_differences(expected: &Value, actual: &Value) -> Vec<String> {
    let scale = actual["scaleFactor"].as_f64().unwrap_or(1.);
    REGIONS
        .iter()
        .filter(|region| **region != "popover")
        .filter(|region| expected["regions"].get(*region).is_some())
        .filter_map(|&region| {
            let want = bounds(expected, region);
            let got = bounds(actual, region);
            let matches = if SNAPPED_REGIONS.contains(&region) {
                want == got || on_device_grid(want, got, scale)
            } else {
                want == got
            };
            (!matches).then(|| format!("{region}: expected {want:?}, got {got:?} at scale {scale}"))
        })
        .collect()
}

fn assert_parity(state: &str) {
    let expected = reference(state);
    let actual = dump(state);
    let mut differences = region_differences(&expected, &actual);
    differences.extend(control_differences(
        &expected,
        &actual,
        &content_controls(state),
    ));
    assert!(
        differences.is_empty(),
        "{state} differs from the reference:\n{}",
        differences.join("\n")
    );
}

#[test]
fn empty_state_matches_the_reference() {
    assert_parity("empty");
}

#[test]
fn projects_sidebar_matches_the_reference() {
    assert_parity("projects");
}

#[test]
fn project_popover_matches_the_reference() {
    assert_parity("project-popover");
}

#[test]
fn model_popover_matches_the_reference() {
    assert_parity("model-popover");
}

#[test]
fn agent_menu_matches_the_reference() {
    assert_parity("agent-menu");
}

#[test]
fn open_session_matches_the_reference() {
    assert_parity("session");
}

#[test]
fn thinking_menu_matches_the_reference() {
    assert_parity("thinking-menu");
}

#[test]
fn streaming_session_matches_the_reference() {
    assert_parity("streaming");
}

/// The server-error state exists only in this app; its regions sit where the empty state's do.
#[test]
fn server_error_keeps_the_empty_state_regions() {
    let differences = region_differences(&dump("empty"), &dump("server-error"));

    assert!(
        differences.is_empty(),
        "server-error regions moved:\n{}",
        differences.join("\n")
    );
}

/// Paths this crate could bake into the binary through `env!` or `include!`.
fn crate_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        fixtures().to_path_buf(),
    ]
}

/// Searches in overlapping chunks through `str::contains`, which runs std's optimized search;
/// a byte loop here compiles unoptimized and takes close to a minute on a debug binary.
fn contains_bytes(haystack: &[u8], needle: &str) -> bool {
    const CHUNK: usize = 32 << 20;
    let overlap = needle.len();
    let mut start = 0;
    while start < haystack.len() {
        let end = (start + CHUNK + overlap).min(haystack.len());
        if String::from_utf8_lossy(&haystack[start..end]).contains(needle) {
            return true;
        }
        start += CHUNK;
    }
    false
}

/// The binary must not depend on, or reveal, the source tree it was built from.
#[test]
fn binary_embeds_no_source_tree_path() {
    let binary = std::fs::read(env!("CARGO_BIN_EXE_oc-app")).expect("oc-app binary reads");
    let found: Vec<String> = crate_paths()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .filter(|needle| contains_bytes(&binary, needle))
        .collect();

    assert!(found.is_empty(), "the oc-app binary contains {found:?}");
}

#[test]
fn composer_edges_compare_on_the_device_grid() {
    let reference_empty = [315.75, 387.6, 720.5, 100.];
    let reference_session = [315.75, 784., 720.5, 100.];

    // Measured on this machine in the headless window, at its 2x scale (0.5 px grid).
    assert!(on_device_grid(
        reference_empty,
        [315.5, 387.5, 720.5, 100.],
        2.
    ));
    assert!(on_device_grid(
        reference_session,
        [315.5, 784., 720.5, 100.],
        2.
    ));
    // A whole logical pixel off is two device pixels at 2x, a real difference.
    assert!(!on_device_grid(
        reference_session,
        [314.5, 784., 720.5, 100.],
        2.
    ));
    assert!(!on_device_grid(
        reference_session,
        [315.5, 785., 720.5, 100.],
        2.
    ));
}
