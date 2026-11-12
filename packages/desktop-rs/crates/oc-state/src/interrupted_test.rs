use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{directory, form_from, opened};
use crate::ids::SessionId;
use crate::recoverable::RecoveryEvidence;
use crate::state::State;

fn loaded_history(error: serde_json::Value) -> (State, SessionId) {
    let messages = vec![
        serde_json::from_value(json!({"type": "idle", "id": "msg_idle", "outcome": "interrupted"}))
            .expect("idle"),
        serde_json::from_value(json!({
            "type": "assistant",
            "id": "msg_ask",
            "content": [{"type": "tool", "id": "tool_ask", "name": "question",
                "state": {"status": "error", "input": {}, "error": error}}]
        }))
        .expect("assistant"),
        serde_json::from_value(json!({"type": "user", "id": "msg_user"})).expect("user"),
    ];
    let form = form_from(json!({
        "id": "frm_loaded",
        "sessionID": "ses_q",
        "metadata": {"kind": "question", "tool": {"messageID": "msg_ask", "id": "tool_ask"}},
        "fields": [{"key": "q0", "type": "string"}]
    }));
    let mut state = opened("ses_q");
    let session = SessionId::from("ses_q");
    state.apply(Action::MessagesLoaded {
        session: session.clone(),
        messages,
    });
    state.forms_loaded(&directory(), vec![form]);
    state.forms_loaded(&directory(), Vec::new());
    (state, session)
}

#[test]
fn a_loaded_interrupted_part_with_no_type_gets_the_local_mark() {
    let (state, session) = loaded_history(json!({"message": "Interrupted"}));
    let recovered = state.recovered_questions(&session);
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].evidence, RecoveryEvidence::LocalInterruptMark);
}

#[test]
fn a_loaded_interrupted_part_with_a_type_is_not_marked_locally() {
    let (state, session) = loaded_history(json!({"message": "Interrupted", "type": "aborted"}));
    assert!(state.recovered_questions(&session).is_empty());
}
