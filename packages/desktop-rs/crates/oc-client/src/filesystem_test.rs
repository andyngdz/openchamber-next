use super::{Home, MkdirRequest, MkdirResult};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn text(name: &str) -> String {
    let path = format!("{FIXTURES}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"))
}

#[test]
fn the_captured_home_answers_its_four_roots() {
    let home: Home = serde_json::from_str(&text("fs-home.json")).expect("decode home");

    assert!(home.home.ends_with("/state/home"));
    assert!(home.chats_root.ends_with("/oc-data/chats"));
    assert_eq!(
        home.canonical_chats_root.as_deref(),
        Some(home.chats_root.as_str())
    );
    assert!(
        home.canonical_legacy_chats_root
            .as_deref()
            .is_some_and(|root| root.ends_with("/.config/openchamber/chats"))
    );
}

#[test]
fn the_captured_mkdir_request_and_answer_decode() {
    let request: serde_json::Value =
        serde_json::from_str(&text("fs-mkdir.request.json")).expect("decode mkdir request");
    let result: MkdirResult =
        serde_json::from_str(&text("fs-mkdir.json")).expect("decode mkdir answer");

    let path = request["path"].as_str().expect("path");
    assert!(path.contains("/chats/2026-01-02/session-"));
    assert!(result.success);
    assert_eq!(result.path, path);
}

#[test]
fn a_mkdir_request_serializes_to_the_captured_body() {
    let captured: serde_json::Value =
        serde_json::from_str(&text("fs-mkdir.request.json")).expect("captured body");
    let request = MkdirRequest {
        path: captured["path"].as_str().expect("path").to_owned(),
    };

    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}
