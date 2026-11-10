//! Reading the selection state and the send comparison.

use oc_client::types::SessionRecord;

use super::project_defaults;
use crate::action::Action;
use crate::fixture_test::{api_settings, directory, model_ref, sidebar_session};
use crate::ids::SessionId;
use crate::selection::{EffortChoice, ModelKey, Selection, SelectionAction, SelectionSource};
use crate::sidebar_state::SidebarAction;
use crate::state::State;

fn open(state: &mut State, id: &str) {
    let session = SessionId::from(id);
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        id,
        &directory(),
        1,
    )]));
    state.apply(Action::OpenSession(session));
}

fn desired(variant: Option<EffortChoice>) -> Selection {
    Selection {
        agent: Some("build".to_owned()),
        model: Some(ModelKey::new("perf", "stream-600cps")),
        variant,
        variant_override: None,
        agent_source: SelectionSource::Manual,
        model_source: SelectionSource::Manual,
        model_pick: super::super::value::ModelPick::None,
    }
}

#[test]
fn project_defaults_come_from_its_raw_settings_entry() {
    let settings = serde_json::from_value(serde_json::json!({
        "projects": [{
            "id": "p", "path": "/p/app",
            "defaultAgent": "plan", "defaultModel": "perf/stream-600cps",
            "defaultVariant": "high",
        }]
    }))
    .expect("settings");
    let defaults = project_defaults(&settings, "/p/app");
    assert_eq!(defaults.agent, Some("plan"));
    assert_eq!(defaults.model, Some("perf/stream-600cps"));
    assert_eq!(defaults.variant, Some("high"));
    assert_eq!(project_defaults(&settings, "/other"), Default::default());
}

#[test]
fn an_open_session_without_a_record_has_no_selection_yet() {
    let mut state = State::new(directory());
    opportunity(&mut state);
    assert!(state.current_selection().is_none());
}

fn opportunity(state: &mut State) {
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(api_settings())));
    open(state, "ses_1");
}

#[test]
fn the_send_comparison_reads_the_record_held_in_state() {
    let mut state = State::new(directory());
    opportunity(&mut state);
    let session = SessionId::from("ses_1");
    state.apply_selection(SelectionAction::SessionRecordLoaded {
        session: session.clone(),
        record: SessionRecord {
            id: "ses_1".to_owned(),
            agent: Some("build".to_owned()),
            model: Some(model_ref("perf", "stream-600cps", Some("high"))),
        },
    });
    assert!(state.session_record(&session).is_some());

    let steps = state.resolve_send_steps(
        &session,
        &desired(Some(EffortChoice::Named("high".to_owned()))),
        &SessionRecord {
            id: "ses_1".to_owned(),
            agent: Some("build".to_owned()),
            model: None,
        },
    );
    assert_eq!(steps.agent, None);
    assert_eq!(steps.model, None);

    let steps = state.resolve_send_steps(
        &session,
        &desired(Some(EffortChoice::Default)),
        &SessionRecord {
            id: "ses_1".to_owned(),
            agent: Some("plan".to_owned()),
            model: None,
        },
    );
    assert_eq!(steps.agent.as_deref(), Some("build"));
    assert_eq!(steps.model, Some(model_ref("perf", "stream-600cps", None)));
}
