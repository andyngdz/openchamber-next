use oc_client::form_field::FormField;
use serde_json::json;

use crate::fixture_test::{api_forms, form_from};
use crate::form::{
    ExternalLinks, FormValues, build_form_answer, initial_form_values, missing_required_keys,
    visible_fields,
};
use crate::websearch::{WebSearchConsentStep, read_web_search_consent};

fn fields(value: serde_json::Value) -> Vec<FormField> {
    form_from(json!({"id": "frm_test", "sessionID": "ses_test", "fields": value})).fields
}

fn yes_no(key: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut field = json!({
        "key": key,
        "type": "string",
        "options": [{"value": "yes", "label": "Yes"}, {"value": "no", "label": "No"}],
    });
    if let (Some(base), Some(extra)) = (field.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    field
}

fn selected(values: &mut FormValues, key: &str, picked: &[&str]) {
    let entry = values.entry(key.to_owned()).or_default();
    entry.selected = picked.iter().map(|value| (*value).to_owned()).collect();
}

#[test]
fn a_hidden_field_hides_its_dependents_even_with_defaults() {
    let chain = fields(json!([
        yes_no("a", json!({})),
        yes_no(
            "b",
            json!({"when": [{"key": "a", "op": "eq", "value": "yes"}]})
        ),
        yes_no(
            "c",
            json!({"default": "yes", "when": [{"key": "b", "op": "eq", "value": "yes"}]})
        ),
    ]));
    let mut values = initial_form_values(&chain, ExternalLinks::Unacknowledged);
    assert_eq!(
        visible_fields(&chain, &values)
            .iter()
            .map(FormField::key)
            .collect::<Vec<_>>(),
        ["a"]
    );
    selected(&mut values, "a", &["no"]);
    assert_eq!(build_form_answer(&chain, &values).len(), 1);
    assert_eq!(visible_fields(&chain, &values).len(), 1);
}

#[test]
fn the_chain_opens_step_by_step_as_answers_arrive() {
    let chain = fields(json!([
        yes_no("a", json!({})),
        yes_no(
            "b",
            json!({"when": [{"key": "a", "op": "eq", "value": "yes"}]})
        ),
        yes_no(
            "c",
            json!({"default": "yes", "when": [{"key": "b", "op": "eq", "value": "yes"}]})
        ),
    ]));
    let mut values = initial_form_values(&chain, ExternalLinks::Unacknowledged);
    selected(&mut values, "a", &["yes"]);
    assert_eq!(visible_fields(&chain, &values).len(), 2);
    selected(&mut values, "b", &["yes"]);
    assert_eq!(visible_fields(&chain, &values).len(), 3);
    assert_eq!(build_form_answer(&chain, &values).len(), 3);
}

#[test]
fn neq_against_an_unanswered_field_is_false() {
    let fields = fields(json!([
        {"key": "name", "type": "string"},
        {"key": "why", "type": "string", "when": [{"key": "name", "op": "neq", "value": "skip"}]},
    ]));
    let mut values = initial_form_values(&fields, ExternalLinks::Unacknowledged);
    values.get_mut("why").expect("why").text = "typed before the gate opened".to_owned();
    assert!(build_form_answer(&fields, &values).is_empty());
    values.get_mut("name").expect("name").text = "Ada".to_owned();
    let answer = build_form_answer(&fields, &values);
    assert_eq!(answer.len(), 2);
}

#[test]
fn a_required_field_behind_a_closed_gate_is_not_missing() {
    let fields = fields(json!([
        yes_no("a", json!({})),
        {"key": "detail", "type": "string", "required": true, "when": [{"key": "a", "op": "eq", "value": "yes"}]},
    ]));
    let mut values = initial_form_values(&fields, ExternalLinks::Unacknowledged);
    selected(&mut values, "a", &["no"]);
    assert!(missing_required_keys(&fields, &values).is_empty());
    selected(&mut values, "a", &["yes"]);
    assert_eq!(missing_required_keys(&fields, &values), ["detail"]);
}

#[test]
fn a_multiselect_clause_matches_any_selected_entry() {
    let fields = fields(json!([
        {"key": "areas", "type": "multiselect", "options": [{"value": "ui", "label": "UI"}, {"value": "api", "label": "API"}]},
        {"key": "route", "type": "string", "when": [{"key": "areas", "op": "eq", "value": "api"}]},
    ]));
    let mut values = initial_form_values(&fields, ExternalLinks::Unacknowledged);
    selected(&mut values, "areas", &["ui"]);
    assert_eq!(visible_fields(&fields, &values).len(), 1);
    selected(&mut values, "areas", &["ui", "api"]);
    assert_eq!(visible_fields(&fields, &values).len(), 2);
}

#[test]
fn external_links_must_be_acknowledged() {
    let fields = fields(json!([
        {"key": "elicitation", "type": "external", "url": "https://example.test/auth"},
    ]));
    let unseen = initial_form_values(&fields, ExternalLinks::Unacknowledged);
    assert_eq!(missing_required_keys(&fields, &unseen), ["elicitation"]);
    assert!(build_form_answer(&fields, &unseen).is_empty());
    let seen = initial_form_values(&fields, ExternalLinks::Acknowledged);
    assert!(missing_required_keys(&fields, &seen).is_empty());
    assert_eq!(build_form_answer(&fields, &seen).len(), 1);
}

#[test]
fn defaults_seed_each_field_type() {
    let fields = fields(json!([
        {"key": "choice", "type": "string", "options": [{"value": "a", "label": "A"}], "default": "a"},
        {"key": "free", "type": "string", "options": [{"value": "a", "label": "A"}], "default": "typed"},
        {"key": "amount", "type": "number", "default": 3},
        {"key": "whole", "type": "integer", "default": "Infinity"},
        {"key": "ship", "type": "boolean", "default": true},
        {"key": "areas", "type": "multiselect", "options": [{"value": "ui", "label": "UI"}], "default": ["ui"]},
    ]));
    let values = initial_form_values(&fields, ExternalLinks::Unacknowledged);
    assert_eq!(values["choice"].selected, ["a"]);
    let free = &values["free"];
    assert_eq!(free.text, "typed");
    assert!(free.custom);
    assert_eq!(values["amount"].number, Some(3.0));
    assert_eq!(values["whole"].number, None);
    assert!(values["ship"].boolean);
    assert_eq!(values["areas"].selected, ["ui"]);
}

#[test]
fn the_fixture_form_answers_its_three_fields() {
    let form = api_forms("form-list")
        .into_iter()
        .next()
        .expect("the fixture question form");
    let mut values = initial_form_values(&form.fields, ExternalLinks::Unacknowledged);
    selected(&mut values, "q0", &["First option"]);
    selected(&mut values, "q1", &["Alpha detail"]);
    values.get_mut("q2").expect("q2").text = "a typed answer".to_owned();
    let answer = build_form_answer(&form.fields, &values);
    assert_eq!(answer.len(), 3);
    assert!(answer.contains_key("q0"));
}

#[test]
fn consent_detection_reads_choice_and_provider_steps() {
    let choice = form_from(json!({
        "id": "frm_consent",
        "sessionID": "ses_test",
        "metadata": {"kind": "websearch.provider"},
        "fields": [{"key": "choice", "type": "string", "options": [{"value": "allow", "label": "Allow"}]}],
    }));
    let consent = read_web_search_consent(&choice).expect("choice step");
    assert_eq!(consent.step, WebSearchConsentStep::Choice);
    assert_eq!(consent.options.len(), 1);

    let provider = form_from(json!({
        "id": "frm_provider",
        "sessionID": "ses_test",
        "metadata": {"kind": "websearch.provider"},
        "fields": [{"key": "provider", "type": "string", "options": [{"value": "p", "label": "P"}]}],
    }));
    assert_eq!(
        read_web_search_consent(&provider)
            .expect("provider step")
            .step,
        WebSearchConsentStep::Provider
    );

    let plain = form_from(json!({
        "id": "frm_plain",
        "sessionID": "ses_test",
        "fields": [{"key": "choice", "type": "string", "options": [{"value": "allow", "label": "Allow"}]}],
    }));
    assert!(read_web_search_consent(&plain).is_none());
}
