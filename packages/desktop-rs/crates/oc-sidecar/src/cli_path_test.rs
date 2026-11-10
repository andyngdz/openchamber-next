use std::fs;
use std::path::PathBuf;

use super::resolve_cli_path;
use crate::errors::SidecarError;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oc-sidecar-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[test]
fn override_wins_over_the_walk() {
    let resolved = resolve_cli_path(Some(PathBuf::from("/x/cli.js")), &[PathBuf::from("/")]);
    assert_eq!(resolved.expect("override"), PathBuf::from("/x/cli.js"));
}

#[test]
fn walks_up_to_the_repository_root() {
    let root = scratch_dir("walk");
    let cli = root.join("packages/web/bin/cli.js");
    fs::create_dir_all(cli.parent().expect("parent")).expect("bin dir");
    fs::write(&cli, "").expect("cli file");
    let nested = root.join("packages/desktop-rs/target/debug");
    fs::create_dir_all(&nested).expect("nested dir");

    let resolved = resolve_cli_path(None, &[nested]).expect("found");
    assert_eq!(resolved, cli);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn reports_the_searched_dirs_when_nothing_is_found() {
    let root = scratch_dir("missing");
    let error = resolve_cli_path(None, std::slice::from_ref(&root)).expect_err("not found");
    let SidecarError::CliNotFound { searched } = error else {
        unreachable!("unexpected error {error:?}");
    };
    assert_eq!(searched, vec![root.clone()]);
    fs::remove_dir_all(&root).ok();
}
