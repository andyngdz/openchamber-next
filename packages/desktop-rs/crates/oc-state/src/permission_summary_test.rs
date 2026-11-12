use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::fixture_test::api_permissions;
use crate::permission_summary::{PermissionSummaryKind, summarize_permission};

fn metadata(value: Value) -> BTreeMap<String, Value> {
    serde_json::from_value(value).expect("metadata object")
}

#[test]
fn shows_an_external_directory_without_its_glob_and_names_the_file() {
    let summary = summarize_permission(
        "external_directory",
        &["/home/me/.config/app/themes/*".to_owned()],
        &metadata(json!({
            "filepath": "/home/me/.config/app/themes/jade.json",
            "parentDir": "/home/me/.config/app/themes"
        })),
        true,
    );
    assert_eq!(summary.kind, PermissionSummaryKind::ExternalDirectory);
    assert_eq!(summary.targets.len(), 1);
    assert_eq!(summary.targets[0].value, "/home/me/.config/app/themes");
    assert!(summary.targets[0].is_path);
    assert_eq!(summary.targets[0].file.as_deref(), Some("jade.json"));
}

#[test]
fn keeps_the_raw_details_for_a_tool_it_cannot_explain() {
    let summary = summarize_permission(
        "linear_create_issue",
        &["*".to_owned()],
        &metadata(json!({"title": "x"})),
        true,
    );
    assert_eq!(summary.kind, PermissionSummaryKind::Tool);
    assert_eq!(summary.tool.as_deref(), Some("linear_create_issue"));
    assert!(summary.targets.is_empty());
    assert!(!summary.metadata_explained);
}

#[test]
fn reports_a_search_scope_and_ignores_malformed_metadata() {
    assert_eq!(
        summarize_permission(
            "grep",
            &["TODO".to_owned()],
            &metadata(json!({"path": "src"})),
            true
        )
        .scope
        .as_deref(),
        Some("src")
    );
    assert!(
        summarize_permission(
            "glob",
            &["*.ts".to_owned()],
            &metadata(json!({"path": 42})),
            true
        )
        .scope
        .is_none()
    );
}

#[test]
fn reads_the_captured_shell_and_edit_requests() {
    let requests = api_permissions("permission-request-list");
    let captured = requests.first().expect("the captured request");
    let summary = summarize_permission(
        &captured.action,
        &captured.resources,
        &captured.metadata,
        !captured.metadata.is_empty(),
    );
    assert_eq!(summary.kind, PermissionSummaryKind::Shell);
    assert!(summary.targets.is_empty());
}
