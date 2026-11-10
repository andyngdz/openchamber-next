use serde_json::Value;

use super::{Project, ProjectPatch};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn settings() -> Value {
    let path = format!("{FIXTURES}/settings.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).expect("settings json")
}

fn first_project() -> Project {
    serde_json::from_value(settings()["projects"][0].clone()).expect("first project")
}

#[test]
fn a_captured_project_exposes_its_typed_fields() {
    let project = first_project();

    assert!(project.id().is_some_and(|id| id.starts_with("path_")));
    assert_eq!(project.label(), Some("Fixture project"));
    assert_eq!(project.default_agent(), None);
    assert_eq!(project.default_model(), None);
    assert_eq!(project.added_at(), Some(1_799_999_998_000));
    assert_eq!(project.last_opened_at(), Some(1_799_999_999_000));
    assert_eq!(project.sidebar_collapsed(), Some(false));
}

#[test]
fn a_patch_keeps_every_field_it_does_not_name() {
    let project = first_project();
    let before = serde_json::to_value(&project).expect("serialize");
    let patch = ProjectPatch {
        last_opened_at: Some(1_800_000_001_000),
        sidebar_collapsed: Some(true),
        ..ProjectPatch::default()
    };

    let patched = serde_json::to_value(project.with_patch(&patch)).expect("serialize");

    assert_eq!(patched["lastOpenedAt"], 1_800_000_001_000_u64);
    assert_eq!(patched["sidebarCollapsed"], true);
    for key in [
        "id",
        "path",
        "label",
        "icon",
        "iconBackground",
        "color",
        "addedAt",
    ] {
        assert_eq!(patched[key], before[key], "{key} changed");
    }
}

#[test]
fn a_patch_can_replace_a_typed_text_field() {
    let patch = ProjectPatch {
        label: Some("Renamed".to_owned()),
        default_agent: Some("build".to_owned()),
        default_model: Some("perf/stream-600cps".to_owned()),
        default_variant: Some("high".to_owned()),
        ..ProjectPatch::default()
    };

    let patched = first_project().with_patch(&patch);

    assert_eq!(patched.label(), Some("Renamed"));
    assert_eq!(patched.default_agent(), Some("build"));
    assert_eq!(patched.default_model(), Some("perf/stream-600cps"));
    assert_eq!(patched.default_variant(), Some("high"));
}
