//! Layout parity with the Electron desktop (`fixtures/reference/`), and a check that the
//! binary carries no build-machine path.
//!
//! The parity cases open a window, so they run only with `OC_DESKTOP_UI=1`:
//! `OC_DESKTOP_UI=1 cargo test -p oc-app --test parity`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError, mpsc};
use std::time::Duration;

use serde_json::Value;

const REGIONS: [&str; 6] = [
    "sidebar",
    "header",
    "chatColumn",
    "composer",
    "workStatusPanel",
    "iconRail",
];

fn fixtures() -> &'static Path {
    Path::new(env!("OC_DESKTOP_FIXTURES"))
}

fn ui_enabled() -> bool {
    std::env::var("OC_DESKTOP_UI").is_ok_and(|value| value == "1")
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

/// Longest a dump may take; a window that never gets a frame fails here instead of hanging.
const DUMP_LIMIT: Duration = Duration::from_secs(60);

/// Runs `oc-app --dump-layout` for one state fixture and returns the dump.
///
/// Dumps run one at a time: GPUI draws only when the compositor sends a frame, and Wayland
/// stops sending frames to a window another window covers, so parallel test windows can
/// starve each other.
fn dump(state: &str) -> Value {
    static ONE_WINDOW: Mutex<()> = Mutex::new(());
    static DUMPS: AtomicUsize = AtomicUsize::new(0);
    let _window = ONE_WINDOW.lock().unwrap_or_else(PoisonError::into_inner);
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

/// The region whose reference bounds are not on any device-pixel grid: React centers the
/// composer in the chat column (x 315.75, width 720.5) and the empty state places it at
/// y 387.6. GPUI snaps every edge to the device grid, so it is compared on that grid.
const SNAPPED_REGION: &str = "composer";

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
/// composer, which matches on the device grid of the scale the dump ran at.
fn region_differences(expected: &Value, actual: &Value) -> Vec<String> {
    let scale = actual["scaleFactor"].as_f64().unwrap_or(1.);
    REGIONS
        .iter()
        .filter_map(|&region| {
            let want = bounds(expected, region);
            let got = bounds(actual, region);
            let matches = if region == SNAPPED_REGION {
                want == got || on_device_grid(want, got, scale)
            } else {
                want == got
            };
            (!matches).then(|| format!("{region}: expected {want:?}, got {got:?} at scale {scale}"))
        })
        .collect()
}

fn keys(layout: &Value, region: &str) -> Vec<String> {
    layout["regions"][region]["controls"]
        .as_array()
        .map(|controls| {
            controls
                .iter()
                .filter_map(|control| control["key"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The key without its `#n`, and whether it had one.
fn base_key(key: &str) -> (&str, bool) {
    match key.rsplit_once('#') {
        Some((base, ordinal)) if ordinal.parse::<usize>().is_ok() => (base, true),
        _ => (key, false),
    }
}

/// Reference controls missing from the dump, and regions whose shared controls are out of order.
///
/// A key with `#n` (one per tool row) is compared by its base key. React virtualizes the
/// timeline (`MessageList.tsx`, `@legendapp/list`), so the reference lists only the rows
/// mounted around the viewport and counts `#n` from the first mounted one; the spike draws
/// every row. The reference's repeated rows must appear, in order, as one unbroken run of the
/// dump's repeated rows.
fn control_differences(
    expected: &Value,
    actual: &Value,
    skipped: &HashSet<(String, String)>,
) -> Vec<String> {
    let mut differences = Vec::new();
    for region in REGIONS {
        let wanted_all: Vec<String> = keys(expected, region)
            .into_iter()
            .filter(|key| !skipped.contains(&(region.to_owned(), key.clone())))
            .collect();
        let dumped_all = keys(actual, region);
        let (wanted_rows, wanted): (Vec<String>, Vec<String>) =
            wanted_all.into_iter().partition(|key| base_key(key).1);
        differences.extend(single_differences(region, &wanted, &dumped_all));
        differences.extend(run_difference(region, &wanted_rows, &dumped_all));
    }
    differences
}

/// Controls whose key does not repeat: each must be in the dump, in the reference order.
fn single_differences(region: &str, wanted: &[String], dumped_all: &[String]) -> Option<String> {
    let wanted_set: HashSet<&String> = wanted.iter().collect();
    let dumped: Vec<&String> = dumped_all
        .iter()
        .filter(|key| wanted_set.contains(key))
        .collect();
    let dumped_set: HashSet<&String> = dumped.iter().copied().collect();
    let missing: Vec<&String> = wanted
        .iter()
        .filter(|key| !dumped_set.contains(key))
        .collect();
    if !missing.is_empty() {
        return Some(format!("{region}: missing {missing:?}"));
    }
    let in_order = dumped.iter().copied().eq(wanted.iter());
    (!in_order)
        .then(|| format!("{region}: order differs\n  expected {wanted:?}\n  got      {dumped:?}"))
}

/// Repeated rows: their base keys must be one unbroken run of the dump's repeated rows.
fn run_difference(region: &str, wanted_rows: &[String], dumped_all: &[String]) -> Option<String> {
    if wanted_rows.is_empty() {
        return None;
    }
    let wanted: Vec<&str> = wanted_rows.iter().map(|key| base_key(key).0).collect();
    let dumped: Vec<&str> = dumped_all
        .iter()
        .map(|key| base_key(key))
        .filter_map(|(base, repeated)| repeated.then_some(base))
        .collect();
    let found = dumped.windows(wanted.len()).any(|run| run == wanted);
    (!found).then(|| {
        format!("{region}: rows are not one run of the dump\n  expected {wanted:?}\n  got      {dumped:?}")
    })
}

fn assert_parity(state: &str) {
    if !ui_enabled() {
        return;
    }
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
fn open_session_matches_the_reference() {
    assert_parity("session");
}

#[test]
fn streaming_session_matches_the_reference() {
    assert_parity("streaming");
}

/// The server-error state exists only in this app; its regions sit where the empty state's do.
#[test]
fn server_error_keeps_the_empty_state_regions() {
    if !ui_enabled() {
        return;
    }
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
fn repeated_rows_match_a_window_of_the_dump() {
    let owned = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>();
    let dumped = owned(&["c/a#1", "c/b#1", "c/a#2", "c/b#2", "c/a#3", "c/More"]);

    // React mounted rows 2 to 4 and numbered them from 1.
    assert_eq!(
        run_difference("c", &owned(&["c/b#1", "c/a#1", "c/b#2"]), &dumped),
        None
    );
    // The same rows out of order, or a row the dump does not have, still differ.
    assert!(run_difference("c", &owned(&["c/a#1", "c/a#2"]), &dumped).is_some());
    assert!(run_difference("c", &owned(&["c/z#1", "c/z#2"]), &dumped).is_some());
    // A name holding `#` that is not an ordinal is a single key.
    assert_eq!(base_key("c/Issue #x"), ("c/Issue #x", false));
}

#[test]
fn composer_edges_compare_on_the_device_grid() {
    let reference_empty = [315.75, 387.6, 720.5, 100.];
    let reference_session = [315.75, 784., 720.5, 100.];

    // Measured on this machine at scale 1.25 (0.8 px grid).
    assert!(on_device_grid(
        reference_empty,
        [316., 387.2, 720.8, 100.],
        1.25
    ));
    assert!(on_device_grid(
        reference_session,
        [315.2, 784., 720.8, 100.],
        1.25
    ));
    // A whole logical pixel off is two device pixels at 1.25, a real difference.
    assert!(!on_device_grid(
        reference_session,
        [314.4, 784., 720.8, 100.],
        1.25
    ));
    assert!(!on_device_grid(
        reference_session,
        [315.2, 785.6, 720.8, 100.],
        1.25
    ));
}
