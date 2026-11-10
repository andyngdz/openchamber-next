//! The state wiring of the picker rules and the agent memory.

use oc_client::types::SessionRecord;
use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{
    agent, catalog_with, chat_model, model_ref, provider, settings, sidebar_session,
};
use crate::ids::SessionId;
use crate::selection::{EffortChoice, ModelKey, SelectionAction, SelectionSource};
use crate::sidebar_state::SidebarAction;
use crate::state::State;

const DIRECTORY: &str = "/p";

fn catalog() -> crate::selection::Catalog {
    catalog_with(
        vec![provider("p")],
        vec![
            chat_model("p", "base", &[]),
            chat_model("p", "plan", &[]),
            chat_model("p", "other", &[]),
            chat_model("p", "think", &["low", "high"]),
        ],
        vec![
            agent(
                "build",
                oc_client::catalog::AgentMode::Primary,
                false,
                Some(model_ref("p", "base", None)),
            ),
            agent(
                "plan",
                oc_client::catalog::AgentMode::Primary,
                false,
                Some(model_ref("p", "plan", None)),
            ),
        ],
        crate::fixture_test::config(None, None),
    )
}

fn loaded() -> State {
    let mut state = State::new(DIRECTORY);
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings(json!({
        "defaultAgent": "build",
        "defaultModel": "p/base",
    })))));
    state.apply_selection(SelectionAction::CatalogLoaded {
        directory: DIRECTORY.to_owned(),
        catalog: catalog(),
    });
    state
}

fn open_with_record(state: &mut State, agent: &str, model: (&str, &str), variant: Option<&str>) {
    let session = SessionId::from("ses_1");
    state.apply(Action::SessionsLoaded(vec![sidebar_session(
        "ses_1", DIRECTORY, 1,
    )]));
    state.apply(Action::OpenSession(session.clone()));
    state.apply_selection(SelectionAction::SessionRecordLoaded {
        session,
        record: SessionRecord {
            id: "ses_1".to_owned(),
            agent: Some(agent.to_owned()),
            model: Some(model_ref(model.0, model.1, variant)),
        },
    });
}

#[test]
fn a_model_pick_writes_recent_and_last_selected_and_marks_the_pick_live() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickModel {
        provider_id: "p".to_owned(),
        model_id: "other".to_owned(),
    });
    let selection = state.current_selection().expect("selection");
    assert_eq!(selection.model(), Some(&ModelKey::new("p", "other")));
    assert_eq!(selection.model_source(), SelectionSource::Manual);

    let patch = state.take_settings_patch().expect("patch");
    assert_eq!(patch.recent_models.expect("recent")[0].model_id, "other");
    assert_eq!(patch.last_selected_model.as_deref(), Some("p/other"));
}

#[test]
fn a_model_the_catalog_does_not_offer_is_ignored() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickModel {
        provider_id: "p".to_owned(),
        model_id: "missing".to_owned(),
    });
    let selection = state.current_selection().expect("selection");
    assert_eq!(selection.model(), Some(&ModelKey::new("p", "base")));
    assert!(state.take_settings_patch().is_none());
}

#[test]
fn an_agent_pick_applies_its_pin_and_prepends_recent_agents() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    let selection = state.current_selection().expect("selection");
    assert_eq!(selection.agent(), Some("plan"));
    assert_eq!(selection.model(), Some(&ModelKey::new("p", "plan")));
    let patch = state.take_settings_patch().expect("patch");
    assert_eq!(patch.recent_agents.expect("recent")[0], "plan");
}

#[test]
fn a_variant_pick_writes_last_used_efforts_and_a_default_writes_null() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickModel {
        provider_id: "p".to_owned(),
        model_id: "think".to_owned(),
    });
    state.take_settings_patch();

    state.apply_selection(SelectionAction::PickVariant(Some("high".to_owned())));
    let patch = state.take_settings_patch().expect("patch");
    let efforts = patch.last_used_efforts.expect("efforts");
    assert_eq!(efforts.get("p/think"), Some(&Some("high".to_owned())));
    assert_eq!(
        state.current_selection().and_then(|s| s.variant()),
        Some(&EffortChoice::Named("high".to_owned()))
    );

    state.apply_selection(SelectionAction::PickVariant(None));
    let patch = state.take_settings_patch().expect("patch");
    let efforts = patch.last_used_efforts.expect("efforts");
    assert_eq!(efforts.get("p/think"), Some(&None));
}

#[test]
fn a_send_records_the_level_the_session_was_on() {
    let mut state = loaded();
    open_with_record(&mut state, "build", ("p", "think"), Some("high"));
    state.apply_selection(SelectionAction::SendSucceeded);
    let patch = state.take_settings_patch().expect("patch");
    let efforts = patch.last_used_efforts.expect("efforts");
    assert_eq!(efforts.get("p/think"), Some(&Some("high".to_owned())));
}

#[test]
fn a_favorite_model_and_agent_toggle_through_the_state() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::ToggleFavoriteModel {
        provider_id: "p".to_owned(),
        model_id: "think".to_owned(),
    });
    let patch = state.take_settings_patch().expect("patch");
    assert_eq!(
        patch.favorite_models.expect("favorites")[0].model_id,
        "think"
    );

    state.apply_selection(SelectionAction::ToggleFavoriteAgent("plan".to_owned()));
    let patch = state.take_settings_patch().expect("patch");
    assert_eq!(patch.favorite_agents.expect("favorites"), ["plan"]);
}

#[test]
fn switching_agents_and_back_restores_the_agents_model() {
    let mut state = loaded();
    open_with_record(&mut state, "build", ("p", "base"), None);

    state.apply_selection(SelectionAction::PickModel {
        provider_id: "p".to_owned(),
        model_id: "other".to_owned(),
    });
    state.apply_selection(SelectionAction::SendSucceeded);
    state.take_settings_patch();

    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    assert_eq!(
        state.current_selection().and_then(|s| s.model()),
        Some(&ModelKey::new("p", "plan"))
    );

    state.apply_selection(SelectionAction::PickAgent("build".to_owned()));
    assert_eq!(
        state.current_selection().and_then(|s| s.model()),
        Some(&ModelKey::new("p", "other"))
    );
}

#[test]
fn a_live_pick_survives_an_agent_switch_until_it_is_sent() {
    let mut state = loaded();
    open_with_record(&mut state, "build", ("p", "base"), None);
    state.apply_selection(SelectionAction::PickModel {
        provider_id: "p".to_owned(),
        model_id: "other".to_owned(),
    });
    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    assert_eq!(
        state.current_selection().and_then(|s| s.model()),
        Some(&ModelKey::new("p", "other"))
    );
}

#[test]
fn sessions_keep_their_own_selection() {
    let mut state = loaded();
    open_with_record(&mut state, "build", ("p", "base"), None);
    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    let first = SessionId::from("ses_1");
    assert_eq!(
        state.session_selection(&first).and_then(|s| s.agent()),
        Some("plan")
    );
    assert_eq!(
        state.draft().selection.agent(),
        Some("build"),
        "the draft keeps its own selection"
    );
    assert_eq!(
        state.current_selection().and_then(|s| s.agent()),
        Some("plan")
    );
}

#[test]
fn an_agent_already_in_recent_agents_leaves_the_patch_empty() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    assert!(state.take_settings_patch().is_some());
    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    assert!(
        state.take_settings_patch().is_none(),
        "the agent is already first in Recent"
    );
}

#[test]
fn the_draft_and_an_open_session_do_not_share_a_selection() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickAgent("plan".to_owned()));
    assert_eq!(state.draft().selection.agent(), Some("plan"));
    open_with_record(&mut state, "build", ("p", "base"), None);
    assert_eq!(
        state.current_selection().and_then(|s| s.agent()),
        Some("build")
    );
    assert_eq!(state.draft().selection.agent(), Some("plan"));
}
