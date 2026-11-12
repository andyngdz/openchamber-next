use serde_json::json;

use crate::fixture_test::{api_forms, form_from};
use crate::recoverable::{
    ErrorOrigin, QuestionAskOutcome, classify_question_tool_error, question_tool_link,
};

#[test]
fn reads_turn_death_shutdown_and_dismissal_apart() {
    assert_eq!(
        classify_question_tool_error(
            "Tool execution interrupted",
            Some("aborted"),
            ErrorOrigin::Server
        ),
        QuestionAskOutcome::TurnDeath
    );
    assert_eq!(
        classify_question_tool_error(
            "Interaction cancelled because the location shut down",
            Some("aborted"),
            ErrorOrigin::Server
        ),
        QuestionAskOutcome::Shutdown
    );
    assert_eq!(
        classify_question_tool_error("Step interrupted", Some("aborted"), ErrorOrigin::Server),
        QuestionAskOutcome::Unresolved
    );
    assert_eq!(
        classify_question_tool_error(
            "form cancelled",
            Some("tool.execution"),
            ErrorOrigin::Server
        ),
        QuestionAskOutcome::Dismissed
    );
    assert_eq!(
        classify_question_tool_error("Interrupted", None, ErrorOrigin::Local),
        QuestionAskOutcome::TurnDeath
    );
}

#[test]
fn fails_closed_on_unrecognized_evidence() {
    assert_eq!(
        classify_question_tool_error("something else", Some("aborted"), ErrorOrigin::Server),
        QuestionAskOutcome::Unresolved
    );
    assert_eq!(
        classify_question_tool_error("something else", None, ErrorOrigin::Server),
        QuestionAskOutcome::Unresolved
    );
    assert_eq!(
        classify_question_tool_error("Interrupted", None, ErrorOrigin::Server),
        QuestionAskOutcome::Unresolved
    );
}

#[test]
fn links_the_captured_question_form_to_its_tool_call() {
    let form = api_forms("form-list")
        .into_iter()
        .find(|form| form.session_id != "global")
        .expect("the captured question form");
    let link = question_tool_link(&form).expect("the question link");
    assert!(!link.message_id.is_empty());
    assert!(!link.tool_id.is_empty());
}

#[test]
fn refuses_consent_global_and_unlinked_forms() {
    let consent = form_from(json!({
        "id": "frm_consent",
        "sessionID": "ses_ask",
        "metadata": {"kind": "websearch.provider"},
        "fields": [{"key": "choice", "type": "string", "options": [{"value": "allow", "label": "Allow"}]}]
    }));
    assert!(question_tool_link(&consent).is_none());

    let global = form_from(json!({
        "id": "frm_global",
        "sessionID": "global",
        "metadata": {"kind": "question", "tool": {"messageID": "msg_ask", "id": "tool_ask"}},
        "fields": [{"key": "q0", "type": "string"}]
    }));
    assert!(question_tool_link(&global).is_none());

    let no_tool = form_from(json!({
        "id": "frm_plain",
        "sessionID": "ses_ask",
        "metadata": {"kind": "question"},
        "fields": [{"key": "q0", "type": "string"}]
    }));
    assert!(question_tool_link(&no_tool).is_none());
}
