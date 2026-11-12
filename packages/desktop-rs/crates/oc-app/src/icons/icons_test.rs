//! The Remix files, the sprite, and the asset source's answers agree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gpui_kit::AssetSource;
use gpui_kit::component::IconNamed;

use super::assets::SUBSTITUTIONS;
use super::remix_files::FILES;
use super::{AppAssets, RemixIcon};
use crate::app::headless::with_settled_window;
use crate::fixture::StateScript;

/// Every asset path asked for since the last call, while a test runs.
static REQUESTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Records one asset path for the render tests.
pub(crate) fn record(path: &str) {
    REQUESTS
        .lock()
        .expect("the asset request log lock")
        .push(path.to_owned());
}

/// Takes the paths recorded since the last call.
pub(crate) fn take() -> Vec<String> {
    std::mem::take(&mut *REQUESTS.lock().expect("the asset request log lock"))
}

/// The checkout root, found by walking up from the crate to the React sprite.
fn repo_root() -> PathBuf {
    let mut directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if directory
            .join("packages/ui/src/components/icon/sprite.ts")
            .is_file()
        {
            return directory;
        }
        assert!(
            directory.pop(),
            "no packages/ui/src/components/icon/sprite.ts above the crate"
        );
    }
}

/// The sprite's symbol names, in file order.
fn sprite_names() -> Vec<String> {
    let path = repo_root().join("packages/ui/src/components/icon/sprite.ts");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut names = Vec::new();
    let mut started = false;
    for raw in source.lines() {
        let line = raw.trim();
        if line.starts_with("export const iconSpriteData") {
            started = true;
            continue;
        }
        if !started {
            continue;
        }
        if line.starts_with("} as const") {
            break;
        }
        let Some(rest) = line.strip_prefix('"') else {
            continue;
        };
        let Some((name, rest)) = rest.split_once("\": `") else {
            continue;
        };
        let Some(content) = rest.strip_suffix("`,").or_else(|| rest.strip_suffix('`')) else {
            continue;
        };
        assert!(
            content.contains("currentColor"),
            "sprite symbol {name} does not keep currentColor"
        );
        names.push(name.to_owned());
    }
    assert!(!names.is_empty(), "sprite.ts holds no symbols");
    names
}

/// The generated SVG file names, without the extension.
fn asset_names() -> BTreeSet<String> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons/remix");
    std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "svg" {
                return None;
            }
            Some(path.file_stem()?.to_string_lossy().into_owned())
        })
        .collect()
}

/// Every `fixtures/states/*.json` file, sorted.
fn states() -> Vec<PathBuf> {
    let directory = Path::new(env!("OC_DESKTOP_FIXTURES")).join("states");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("fixtures/states reads")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    paths
}

/// Renders one state in the headless window and returns every asset path it asked for.
fn render(path: &Path) -> Vec<String> {
    let (state, popover) = StateScript::load(path)
        .expect("state fixture loads")
        .into_parts();
    with_settled_window(state, popover, |_, _, _| Ok(())).expect("state renders headless");
    take()
}

#[test]
fn the_assets_folder_matches_the_sprite_file_for_file() {
    let expected: BTreeSet<String> = sprite_names().into_iter().collect();
    assert_eq!(
        expected,
        asset_names(),
        "assets/icons/remix and sprite.ts list different symbols; run tools/gen-remix-icons.mjs"
    );
}

#[test]
fn every_remix_icon_has_an_asset_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let missing: Vec<String> = RemixIcon::ALL
        .iter()
        .filter(|icon| !root.join(icon.path().as_ref()).is_file())
        .map(|icon| icon.path().to_string())
        .collect();
    assert!(
        missing.is_empty(),
        "Remix icons without a file: {missing:?}"
    );
}

#[test]
fn every_gpui_kit_icon_the_app_draws_is_a_remix_file() {
    for (path, icon) in SUBSTITUTIONS {
        let loaded = AppAssets
            .load(path)
            .unwrap_or_else(|error| panic!("{path} loads: {error}"))
            .unwrap_or_else(|| panic!("{path} has no asset"));
        let expected = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(icon.path().as_ref()),
        )
        .unwrap_or_else(|error| panic!("{} reads: {error}", icon.path()));
        assert!(
            loaded.as_ref() == expected.as_slice(),
            "{path} is not answered with the Remix file {}",
            icon.path()
        );
    }
}

#[test]
fn every_state_asks_only_for_substituted_gpui_kit_icons() {
    let table: Vec<&str> = SUBSTITUTIONS.iter().map(|(path, _)| *path).collect();
    let mut unknown = BTreeSet::new();
    let mut requested = BTreeSet::new();
    for path in states() {
        take();
        for asset in render(&path) {
            if asset.starts_with("icons/remix/") {
                requested.insert(asset);
                continue;
            }
            if asset.starts_with("icons/") && !table.contains(&asset.as_str()) {
                unknown.insert(asset);
            }
        }
    }
    assert!(
        requested.contains("icons/remix/search.svg"),
        "no fixture state drew the named Remix search icon; drew {requested:?}"
    );
    assert!(
        unknown.is_empty(),
        "gpui-kit icon paths missing from SUBSTITUTIONS: {unknown:?}"
    );
}

#[test]
fn every_sprite_symbol_is_embedded() {
    let names: BTreeSet<&str> = FILES.iter().map(|(name, _)| *name).collect();
    let expected: BTreeSet<String> = sprite_names().into_iter().collect();
    let expected: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(expected, names);
}
