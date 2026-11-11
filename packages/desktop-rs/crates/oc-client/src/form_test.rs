//! Forms, their fields, values and bounds, against the captured fixtures.

use serde_json::Value;

use super::{Bound, FormAnswer, FormInfo, FormReplyRequest, FormScalar, FormValue, FormWhenOp};
use crate::form_field::{FormField, FormStringField};
use crate::types::Located;

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn fixture(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

#[test]
fn the_pending_form_list_decodes_into_the_captured_forms() {
    let captured = fixture("form-list.json");
    let located: Located<Vec<FormInfo>> =
        serde_json::from_value(captured.clone()).expect("form list");
    let data = &captured["data"];
    let raw = data.as_array().expect("data");

    assert_eq!(located.data.len(), raw.len());
    for (form, raw) in located.data.iter().zip(raw) {
        assert_eq!(form.id, raw["id"].as_str().expect("id"));
        assert_eq!(form.session_id, raw["sessionID"].as_str().expect("session"));
        assert_eq!(form.title, raw["title"].as_str().expect("title"));
        assert_eq!(form.metadata["kind"], raw["metadata"]["kind"]);
        let raw_fields = raw["fields"].as_array().expect("fields");
        assert_eq!(form.fields.len(), raw_fields.len());
        for (field, raw) in form.fields.iter().zip(raw_fields) {
            assert_eq!(field.key(), raw["key"].as_str().expect("key"));
            assert_eq!(field.kind(), raw["type"].as_str().expect("type"));
        }
    }
    let global = located
        .data
        .iter()
        .find(|form| form.session_id == "global")
        .expect("a location-scoped form");
    assert_eq!(
        global.id,
        data.as_array().expect("data")[1]["id"]
            .as_str()
            .expect("global id")
    );
}

#[test]
fn a_field_of_every_contract_type_decodes() {
    let fields: Vec<FormField> = serde_json::from_value(serde_json::json!([
        {"key": "s", "type": "string", "options": [{"value": "a", "label": "A"}], "custom": true},
        {"key": "n", "type": "number", "minimum": 1, "maximum": 2.5, "default": 1},
        {"key": "i", "type": "integer", "minimum": 1, "maximum": 2, "default": 1},
        {"key": "b", "type": "boolean", "default": true},
        {"key": "m", "type": "multiselect", "options": [{"value": "a", "label": "A"}], "default": ["a"]},
        {"key": "e", "type": "external", "url": "https://example.test"},
    ]))
    .expect("every field type");

    assert!(
        matches!(&fields[0], FormField::String(field) if field.custom && field.options.as_ref().is_some_and(|options| options.len() == 1))
    );
    assert!(
        matches!(&fields[1], FormField::Number(field) if field.minimum == Some(Bound::Number(1.0)))
    );
    assert!(
        matches!(&fields[2], FormField::Integer(field) if field.maximum == Some(Bound::Number(2.0)))
    );
    assert!(matches!(&fields[3], FormField::Boolean(field) if field.default == Some(true)));
    assert!(
        matches!(&fields[4], FormField::Multiselect(field) if field.default.iter().map(String::as_str).collect::<Vec<_>>() == ["a"])
    );
    let external = &fields[5];
    assert!(matches!(external, FormField::External(field) if field.url == "https://example.test"));
    assert_eq!(external.kind(), "external");
}

#[test]
fn number_bounds_accept_numbers_and_the_three_strings() {
    let bound = |value: Value| -> Bound {
        let field: FormField = serde_json::from_value(serde_json::json!({
            "key": "n",
            "type": "number",
            "minimum": value,
        }))
        .expect("number field");
        let FormField::Number(field) = field else {
            panic!("number expected");
        };
        field.minimum.expect("minimum")
    };

    assert_eq!(bound(serde_json::json!(3)), Bound::Number(3.0));
    assert_eq!(bound(serde_json::json!(-1.5)), Bound::Number(-1.5));
    assert_eq!(bound(serde_json::json!("Infinity")), Bound::Infinity);
    assert_eq!(
        bound(serde_json::json!("-Infinity")),
        Bound::NegativeInfinity
    );
    assert_eq!(bound(serde_json::json!("NaN")), Bound::NotANumber);

    let rejected: Result<FormField, _> = serde_json::from_value(serde_json::json!({
        "key": "n",
        "type": "number",
        "minimum": "unbounded",
    }));
    assert!(rejected.is_err());
}

#[test]
fn a_when_clause_compares_against_an_earlier_answer() {
    let field: FormField = serde_json::from_value(serde_json::json!({
        "key": "later",
        "type": "string",
        "when": [{"key": "earlier", "op": "neq", "value": true}],
    }))
    .expect("conditional field");
    let FormField::String(FormStringField { when, .. }) = field else {
        panic!("string expected");
    };

    assert_eq!(when.len(), 1);
    let first = &when[0];
    assert_eq!(first.key, "earlier");
    assert_eq!(first.op, FormWhenOp::Neq);
    assert_eq!(first.value, FormScalar::Bool(true));
}

#[test]
fn an_unknown_field_type_fails_the_form() {
    let result: Result<FormField, _> =
        serde_json::from_value(serde_json::json!({"key": "x", "type": "telepathy"}));
    assert!(result.is_err());
}

#[test]
fn the_captured_answer_decodes_and_serializes_to_its_saved_body() {
    let captured = fixture("form-reply.request.json");
    let captured_answer = captured["answer"].clone();
    let answer: FormAnswer = serde_json::from_value(captured_answer.clone()).expect("form answer");
    let request = FormReplyRequest {
        answer: answer.clone(),
    };

    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
    let raw = captured_answer.as_object().expect("answer");
    for (key, value) in &answer {
        assert_eq!(
            serde_json::to_value(value).expect("value"),
            raw[key],
            "{key}"
        );
    }
    assert!(matches!(
        answer.values().next(),
        Some(FormValue::Scalar(FormScalar::Text(_)))
    ));
}

#[test]
fn the_created_global_form_decodes() {
    let captured = fixture("form-create-global.json");
    let created: crate::types::Data<FormInfo> =
        serde_json::from_value(captured.clone()).expect("created form");
    let raw = &captured["data"];

    assert_eq!(created.data.id, raw["id"].as_str().expect("id"));
    assert_eq!(
        created.data.session_id,
        raw["sessionID"].as_str().expect("session")
    );
    assert_eq!(
        created.data.fields.len(),
        raw["fields"].as_array().expect("fields").len()
    );
}

#[test]
fn a_form_naming_only_its_ids_still_decodes() {
    let form: FormInfo =
        serde_json::from_value(serde_json::json!({"id": "frm_x", "sessionID": "ses_x"}))
            .expect("bare form");

    assert_eq!(form.title, "");
    assert!(form.fields.is_empty());
    assert!(form.metadata.is_empty());
}
