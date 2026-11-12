use serde_json::json;

use crate::fixture_test::form_from;
use crate::form::FormValues;
use crate::form_serializers::serialize_form_answer_as_markdown;

fn value(text: &str) -> crate::form::FieldValue {
    crate::form::FieldValue {
        text: text.to_owned(),
        ..Default::default()
    }
}

#[test]
fn carries_the_question_context_and_the_answer_payload() {
    let form = form_from(json!({
        "id": "frm_plan",
        "sessionID": "ses_test",
        "title": "Plan the work",
        "fields": [
            {"key": "days", "type": "number", "title": "How many days?", "required": true},
            {"key": "ship", "type": "boolean", "title": "Ship it?"},
            {"key": "areas", "type": "multiselect", "title": "Areas", "custom": true, "options": [{"value": "ui", "label": "UI"}]},
            {"key": "color", "type": "string", "title": "Color", "custom": true, "options": [{"value": "red", "label": "Red"}]},
            {"key": "detail", "type": "string", "title": "Only when shipping", "when": [{"key": "ship", "op": "eq", "value": true}]}
        ]
    }));
    let mut values = FormValues::new();
    values.insert(
        "days".to_owned(),
        crate::form::FieldValue {
            number: Some(3.0),
            ..Default::default()
        },
    );
    values.insert(
        "ship".to_owned(),
        crate::form::FieldValue {
            boolean: true,
            ..Default::default()
        },
    );
    values.insert(
        "areas".to_owned(),
        crate::form::FieldValue {
            text: "docs".to_owned(),
            selected: vec!["ui".to_owned()],
            custom: true,
            ..Default::default()
        },
    );
    values.insert(
        "color".to_owned(),
        crate::form::FieldValue {
            text: "teal".to_owned(),
            custom: true,
            ..Default::default()
        },
    );
    values.insert("detail".to_owned(), value("typed but hidden"));
    let answer = crate::form::build_form_answer(&form.fields, &values);

    let text = serialize_form_answer_as_markdown(&form, &answer);
    assert!(text.starts_with("# Plan the work"));
    assert!(text.contains("**Answer:** 3"));
    assert!(text.contains("**Answer:** ui, docs"));
    assert!(text.contains("\"days\":3"));
    assert!(text.contains(
        "{\"days\":3,\"ship\":true,\"areas\":[\"ui\",\"docs\"],\"color\":\"teal\",\"detail\":\"typed but hidden\"}"
    ));
}

#[test]
fn omits_a_field_its_when_clause_hides() {
    let form = form_from(json!({
        "id": "frm_plan",
        "sessionID": "ses_test",
        "title": "Plan the work",
        "fields": [
            {"key": "days", "type": "number", "title": "How many days?"},
            {"key": "ship", "type": "boolean", "title": "Ship it?"},
            {"key": "detail", "type": "string", "title": "Only when shipping", "when": [{"key": "ship", "op": "eq", "value": true}]}
        ]
    }));
    let mut values = FormValues::new();
    values.insert(
        "days".to_owned(),
        crate::form::FieldValue {
            number: Some(3.0),
            ..Default::default()
        },
    );
    values.insert("detail".to_owned(), value("typed but hidden"));
    let text = serialize_form_answer_as_markdown(
        &form,
        &crate::form::build_form_answer(&form.fields, &values),
    );
    assert!(!text.contains("Only when shipping"));
    assert!(text.contains("{\"days\":3,\"ship\":false}"));
}
