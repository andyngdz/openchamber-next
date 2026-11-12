use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::parse;
use crate::theme::{Color, DARK_THEME};

#[derive(Debug, Deserialize)]
struct Fixture {
    theme: String,
    variables: BTreeMap<String, String>,
}

/// The repository root, found the way `oc-sidecar` finds the CLI.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join("packages/web/bin/cli.js").is_file())
        .map(Path::to_path_buf)
        .expect("repository root above the crate")
}

/// A variable's value with `!important` and `var()` references resolved.
fn resolve(variables: &BTreeMap<String, String>, name: &str) -> String {
    let mut value = variables
        .get(name)
        .unwrap_or_else(|| panic!("{name} is missing"))
        .clone();
    loop {
        let trimmed = value.trim().trim_end_matches("!important").trim();
        let Some(reference) = trimmed
            .strip_prefix("var(")
            .and_then(|rest| rest.strip_suffix(')'))
        else {
            return trimmed.to_owned();
        };
        value = variables
            .get(reference.trim())
            .unwrap_or_else(|| panic!("{reference} is missing"))
            .clone();
    }
}

#[test]
fn every_palette_token_equals_its_css_variable() {
    let source = std::fs::read_to_string(
        repo_root().join("packages/desktop-rs/fixtures/theme/palette-variables.json"),
    )
    .expect("palette fixture reads");
    let fixture: Fixture = serde_json::from_str(&source).expect("palette fixture parses");
    assert_eq!(fixture.theme, "openchamber-dark");
    let palette = parse(DARK_THEME).expect("embedded theme parses");
    let tokens: [(&str, Color); 27] = [
        ("--background", palette.background),
        ("--foreground", palette.foreground),
        ("--card", palette.card),
        ("--popover", palette.popover),
        ("--primary", palette.primary),
        ("--primary-foreground", palette.primary_foreground),
        ("--secondary", palette.secondary),
        ("--muted", palette.muted),
        ("--muted-foreground", palette.muted_foreground),
        ("--accent", palette.accent),
        ("--destructive", palette.destructive),
        ("--border", palette.border),
        ("--input", palette.input),
        ("--ring", palette.ring),
        ("--sidebar", palette.sidebar),
        ("--sidebar-accent", palette.sidebar_accent),
        ("--status-error", palette.status_error),
        ("--status-warning", palette.status_warning),
        ("--status-success", palette.status_success),
        ("--status-info", palette.status_info),
        ("--syntax-keyword", palette.syntax_keyword),
        ("--syntax-type", palette.syntax_type),
        ("--syntax-function", palette.syntax_function),
        ("--syntax-number", palette.syntax_number),
        ("--syntax-string", palette.syntax_string),
        ("--syntax-operator", palette.syntax_operator),
        ("--syntax-variable", palette.syntax_variable),
    ];
    for (name, token) in tokens {
        let value = resolve(&fixture.variables, name);
        let expected =
            Color::try_from(value.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(expected, token, "{name}: {value}");
    }
}
