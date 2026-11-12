//! The prompt bodies and accepted answers, against the captured fixtures.

use serde_json::Value;

use super::{Delivery, PromptAccepted, PromptRequest};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn fixture(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

fn prompt_names() -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(FIXTURES)
        .expect("fixtures/api")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("prompt-") && name.ends_with(".json"))
        .filter(|name| !name.ends_with(".request.json"))
        .collect();
    found.sort();
    found
}

#[test]
fn every_prompt_response_decodes_with_the_delivery_the_server_sent() {
    let names = prompt_names();
    assert!(names.len() >= 6, "{names:?}");
    for name in names {
        let captured = fixture(&name);
        let data = &captured["data"];
        let accepted: PromptAccepted = serde_json::from_value(data.clone()).unwrap_or_else(|err| {
            panic!("{name}: {err}");
        });
        assert_eq!(accepted.id, data["id"], "{name}");
        assert_eq!(accepted.session_id, data["sessionID"], "{name}");
        let delivery = data["delivery"].as_str().map(|value| match value {
            "steer" => Delivery::Steer,
            "queue" => Delivery::Queue,
            other => panic!("{name}: unexpected delivery {other}"),
        });
        assert_eq!(accepted.delivery, delivery, "{name}");
    }
}

#[test]
fn a_prompt_without_a_delivery_decodes_as_a_plain_send() {
    let captured = fixture("prompt-text.json");
    let mut data = captured["data"].clone();
    data.as_object_mut()
        .expect("prompt data")
        .remove("delivery");
    let accepted: PromptAccepted = serde_json::from_value(data).expect("prompt");
    assert_eq!(accepted.delivery, None);
}

#[test]
fn the_plain_prompt_request_serializes_to_the_captured_body() {
    let captured = fixture("prompt-text.request.json");
    let request = PromptRequest {
        id: captured["id"].as_str().expect("id").to_owned(),
        text: captured["text"].as_str().expect("text").to_owned(),
        delivery: None,
    };
    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn the_steer_request_serializes_to_the_captured_body() {
    let captured = fixture("prompt-steer.request.json");
    let request = PromptRequest {
        id: captured["id"].as_str().expect("id").to_owned(),
        text: captured["text"].as_str().expect("text").to_owned(),
        delivery: Some(Delivery::Steer),
    };
    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}
