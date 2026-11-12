use serde_json::json;

use crate::fixture_test::form_from;
use crate::websearch::{WebSearchConsentStep, read_web_search_consent};

#[test]
fn reads_the_choice_step() {
    let form = form_from(json!({
        "id": "frm_choice",
        "sessionID": "ses_1",
        "metadata": {"kind": "websearch.provider"},
        "fields": [{"key": "choice", "type": "string", "options": [
            {"value": "allow", "label": "Allow"},
            {"value": "choose", "label": "Choose provider"}
        ]}]
    }));
    let consent = read_web_search_consent(&form).expect("choice consent");
    assert_eq!(consent.step, WebSearchConsentStep::Choice);
    assert_eq!(consent.field_key, "choice");
    assert_eq!(consent.options.len(), 2);
}

#[test]
fn refuses_a_form_without_the_consent_shape() {
    let no_options = form_from(json!({
        "id": "frm_plain",
        "sessionID": "ses_1",
        "metadata": {"kind": "websearch.provider"},
        "fields": [{"key": "choice", "type": "string"}]
    }));
    assert!(read_web_search_consent(&no_options).is_none());

    let two_fields = form_from(json!({
        "id": "frm_two",
        "sessionID": "ses_1",
        "metadata": {"kind": "websearch.provider"},
        "fields": [
            {"key": "choice", "type": "string", "options": [{"value": "allow", "label": "Allow"}]},
            {"key": "provider", "type": "string", "options": [{"value": "p", "label": "P"}]}
        ]
    }));
    assert!(read_web_search_consent(&two_fields).is_none());

    let other_kind = form_from(json!({
        "id": "frm_other",
        "sessionID": "ses_1",
        "metadata": {"kind": "question"},
        "fields": [{"key": "choice", "type": "string", "options": [{"value": "allow", "label": "Allow"}]}]
    }));
    assert!(read_web_search_consent(&other_kind).is_none());
}
