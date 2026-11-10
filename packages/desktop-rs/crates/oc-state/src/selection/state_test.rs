//! The selection state entry point: draft defaults, catalog loads and session records.

use oc_client::types::SessionRecord;
use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{api_catalog, api_settings, directory, settings, sidebar_session};
use crate::ids::SessionId;
use crate::selection::{DraftTarget, EffortChoice, ModelKey, SelectionAction, SelectionSource};
use crate::sidebar_state::SidebarAction;
use crate::state::State;
use crate::status::LoadStatus;

fn loaded(settings: oc_client::Settings) -> State {
    let mut state = State::new(directory());
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings)));
    state
}

#[test]
fn a_fresh_draft_targets_chat_and_holds_nothing() {
    let state = State::new("/home/u");
    assert_eq!(state.draft().target, DraftTarget::Chat);
    assert_eq!(state.draft().selection.agent(), None);
    assert_eq!(state.draft().selection.model(), None);
    assert_eq!(state.current_selection().and_then(|s| s.agent()), None);
}

#[test]
fn a_catalog_load_applies_the_cascade_to_the_draft() {
    let mut state = loaded(api_settings());
    state.apply_selection(SelectionAction::CatalogLoaded {
        directory: directory(),
        catalog: api_catalog(),
    });
    let selection = state.current_selection().expect("draft selection");
    assert_eq!(selection.agent(), Some("capture-primary"));
    assert_eq!(
        selection.model(),
        Some(&ModelKey::new("perf", "stream-600cps"))
    );
    assert_eq!(
        selection.variant(),
        Some(&EffortChoice::Named("high".to_owned()))
    );
    assert_eq!(selection.agent_source(), SelectionSource::Auto);
    assert_eq!(selection.model_source(), SelectionSource::Auto);
}

#[test]
fn a_catalog_for_another_directory_leaves_the_draft_alone() {
    let mut state = loaded(api_settings());
    state.apply_selection(SelectionAction::CatalogLoaded {
        directory: "/elsewhere".to_owned(),
        catalog: api_catalog(),
    });
    assert_eq!(state.current_selection().and_then(|s| s.agent()), None);
    assert!(state.catalog("/elsewhere").is_some());
}

#[test]
fn a_catalog_failure_keeps_the_previous_catalog_under_the_error() {
    let mut state = loaded(api_settings());
    state.apply_selection(SelectionAction::CatalogLoaded {
        directory: directory(),
        catalog: api_catalog(),
    });
    state.apply_selection(SelectionAction::CatalogFailed {
        directory: directory(),
        message: "catalog down".to_owned(),
    });
    assert!(state.catalog(&directory()).is_some());
    assert_eq!(
        state.catalog_load(&directory()),
        Some(&LoadStatus::Failed {
            message: "catalog down".to_owned()
        })
    );
}

#[test]
fn a_session_record_starts_the_selection_and_keeps_its_own_fields() {
    let mut state = loaded(settings(
        json!({"defaultAgent": "build", "defaultModel": "p/one"}),
    ));
    let session = SessionId::from("ses_1");
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        "ses_1",
        &directory(),
        1,
    )]));
    state.apply(Action::OpenSession(session.clone()));
    state.apply_selection(SelectionAction::SessionRecordLoaded {
        session: session.clone(),
        record: SessionRecord {
            id: "ses_1".to_owned(),
            agent: Some("plan".to_owned()),
            model: None,
        },
    });
    let selection = state.current_selection().expect("session selection");
    assert_eq!(selection.agent(), Some("plan"));
    assert_eq!(selection.agent_source(), SelectionSource::Session);
    assert_eq!(selection.model(), Some(&ModelKey::new("p", "one")));
    assert_eq!(selection.model_source(), SelectionSource::Auto);
}

#[test]
fn a_record_for_an_unopened_session_does_not_touch_the_draft() {
    let mut state = loaded(api_settings());
    state.apply_selection(SelectionAction::SessionRecordLoaded {
        session: SessionId::from("ses_other"),
        record: SessionRecord {
            id: "ses_other".to_owned(),
            agent: Some("plan".to_owned()),
            model: None,
        },
    });
    assert_eq!(state.current_selection().and_then(|s| s.agent()), None);
    assert!(
        state
            .session_selection(&SessionId::from("ses_other"))
            .is_some()
    );
}
