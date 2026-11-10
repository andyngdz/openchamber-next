//! The reducer side of the sidebar: settings/home loads, page merging, collapse and Show more.

use serde_json::{Value, json};

use crate::action::Action;
use crate::fixture_test::{
    api_home, api_sessions, api_settings, event, home, settings, sidebar_session,
};
use crate::selection::SelectionAction;
use crate::sidebar::{MoreKey, ProjectGroup, SidebarSectionModel};
use crate::sidebar_state::SidebarAction;
use crate::state::State;

fn loaded(settings: oc_client::Settings) -> State {
    let mut state = State::new("/home/u");
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings)));
    state.apply_sidebar(SidebarAction::HomeLoaded(home(json!({"home": "/home/u"}))));
    state
}

fn page(state: &mut State, name: &str) -> Vec<oc_client::types::Session> {
    let sessions = api_sessions(name);
    state.apply_sidebar(SidebarAction::PageLoaded(sessions.clone()));
    sessions
}

fn chats(model: &crate::SidebarModel) -> usize {
    model
        .sections
        .iter()
        .find_map(|section| match section {
            SidebarSectionModel::Chats { rows, .. } => Some(rows.len()),
            _ => None,
        })
        .unwrap_or_default()
}

fn groups(model: &crate::SidebarModel) -> &[ProjectGroup] {
    model
        .sections
        .iter()
        .find_map(|section| match section {
            SidebarSectionModel::Projects { groups } => Some(groups.as_slice()),
            _ => None,
        })
        .unwrap_or_default()
}

fn project_rows(model: &crate::SidebarModel, index: usize) -> usize {
    groups(model).get(index).map_or(0, |group| group.rows.len())
}

fn pinned_rows(model: &crate::SidebarModel) -> usize {
    model
        .sections
        .iter()
        .find_map(|section| match section {
            SidebarSectionModel::Pinned { rows, .. } => Some(rows.len()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn a_settings_or_home_failure_is_the_sidebar_error() {
    let mut settings_failed = State::new("/home/u");
    settings_failed.apply_sidebar(SidebarAction::SettingsFailed("no settings".to_owned()));
    let model = settings_failed.sidebar_model();
    assert_eq!(model.error.as_deref(), Some("no settings"));
    assert!(model.sections.is_empty());

    let mut home_failed = State::new("/home/u");
    home_failed.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings(
        json!({"projects": []}),
    ))));
    home_failed.apply_sidebar(SidebarAction::HomeFailed("no home".to_owned()));
    let model = home_failed.sidebar_model();
    assert_eq!(model.error.as_deref(), Some("no home"));
    assert!(model.sections.is_empty());
}

#[test]
fn the_first_page_is_shown_early_and_the_complete_list_replaces_it() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));
    assert!(state.settings().is_some());
    assert!(state.home().is_some());
    page(&mut state, "session-list-global-page-1");

    let first = state.sidebar_model();
    assert!(!first.complete);
    assert!(!first.incomplete);
    assert_eq!(chats(&first), 1);

    page(&mut state, "session-list-global-page-2");
    let second = state.sidebar_model();
    assert_eq!(chats(&second), 1);

    let mut all = api_sessions("session-list-global-page-1");
    all.extend(api_sessions("session-list-global-page-2"));
    state.apply_sidebar(SidebarAction::Complete(all));
    let complete = state.sidebar_model();
    assert!(complete.complete);
    assert!(!complete.incomplete);
    assert_eq!(chats(&complete), 1);
    assert_eq!(project_rows(&complete, 0), 0);
}

#[test]
fn the_complete_list_replaces_the_rows_the_first_page_showed() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));
    page(&mut state, "session-list-global-page-1");
    assert_eq!(chats(&state.sidebar_model()), 1);

    state.apply_sidebar(SidebarAction::Complete(api_sessions(
        "session-list-global-page-2",
    )));

    let model = state.sidebar_model();
    assert!(model.complete);
    assert_eq!(chats(&model), 0);
    assert_eq!(project_rows(&model, 0), 0);
}

#[test]
fn a_later_page_failure_keeps_the_rows_and_marks_the_list_incomplete() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));
    page(&mut state, "session-list-global-page-1");

    state.apply_sidebar(SidebarAction::PageFailed("later".to_owned()));

    let model = state.sidebar_model();
    assert_eq!(model.error, None);
    assert!(model.incomplete);
    assert!(!model.complete);
    assert_eq!(chats(&model), 1);

    let mut failed_first = loaded(api_settings());
    failed_first.apply_sidebar(SidebarAction::PageFailed("first".to_owned()));
    let model = failed_first.sidebar_model();
    assert_eq!(model.error.as_deref(), Some("first"));
    assert!(!model.incomplete);
}

#[test]
fn created_renamed_and_deleted_events_reach_the_project_group() {
    let mut state = loaded(settings(
        json!({"projects": [{"id": "p", "path": "/p/app"}]}),
    ));
    state.apply(Action::Event(event(
        "session.created",
        10,
        json!({"sessionID": "ses_1", "location": {"directory": "/p/app"}}),
    )));
    assert_eq!(project_rows(&state.sidebar_model(), 0), 1);

    state.apply(Action::Event(event(
        "session.renamed",
        11,
        json!({"sessionID": "ses_1", "title": "Titled"}),
    )));
    let model = state.sidebar_model();
    assert_eq!(groups(&model)[0].rows[0].title.as_deref(), Some("Titled"));

    state.apply(Action::Event(event(
        "session.execution.started",
        12,
        json!({"sessionID": "ses_1"}),
    )));
    let model = state.sidebar_model();
    assert!(groups(&model)[0].rows[0].busy);

    state.apply(Action::Event(event(
        "session.deleted",
        13,
        json!({"sessionID": "ses_1"}),
    )));
    assert_eq!(project_rows(&state.sidebar_model(), 0), 0);
}

#[test]
fn a_metadata_event_moves_a_session_into_pinned_and_back() {
    let mut state = loaded(settings(
        json!({"projects": [{"id": "p", "path": "/p/app"}]}),
    ));
    state.apply_sidebar(SidebarAction::PageLoaded(vec![sidebar_session(
        "ses_1", "/p/app", 1,
    )]));
    assert_eq!(project_rows(&state.sidebar_model(), 0), 1);

    state.apply(Action::Event(event(
        "session.metadata.updated",
        2,
        json!({"sessionID": "ses_1", "metadata": {"openchamber": {"pinned": {"at": 3}}}}),
    )));
    let model = state.sidebar_model();
    assert_eq!(pinned_rows(&model), 1);
    assert_eq!(project_rows(&model, 0), 0);

    state.apply(Action::Event(event(
        "session.metadata.updated",
        3,
        json!({"sessionID": "ses_1", "metadata": {}}),
    )));
    let model = state.sidebar_model();
    assert_eq!(pinned_rows(&model), 0);
    assert_eq!(project_rows(&model, 0), 1);
}

#[test]
fn a_session_in_an_unknown_directory_is_not_shown() {
    let mut state = loaded(settings(
        json!({"projects": [{"id": "p", "path": "/p/app"}]}),
    ));
    state.apply(Action::Event(event(
        "session.created",
        10,
        json!({"sessionID": "ses_1", "location": {"directory": "/elsewhere"}}),
    )));

    assert!(state.global_sessions().is_empty());
    assert_eq!(project_rows(&state.sidebar_model(), 0), 0);
}

#[test]
fn collapsing_a_project_queues_the_whole_projects_array() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));

    state.apply_sidebar(SidebarAction::ToggleProjectCollapsed(
        "path_L3RtcC9vYy1kZXNrdG9wLXJzLWNhcHR1cmUvcHJvamVjdA".to_owned(),
    ));

    let model = state.sidebar_model();
    assert!(groups(&model)[0].project.collapsed);

    let patch = state.take_settings_patch().expect("patch");
    let projects = patch.projects.expect("projects");
    let object: Value = serde_json::to_value(&projects[0]).expect("json");
    assert_eq!(object["sidebarCollapsed"], true);
    assert_eq!(object["color"], "#4F46E5");
    assert_eq!(object["icon"], "rocket");
    assert!(state.take_settings_patch().is_none());
}

#[test]
fn show_more_extends_the_row_list_the_view_reads() {
    let mut state = loaded(settings(
        json!({"projects": [{"id": "p", "path": "/p/app"}]}),
    ));
    for index in 0..10 {
        state.apply_sidebar(SidebarAction::PageLoaded(vec![sidebar_session(
            &format!("ses_{index}"),
            "/p/app",
            index,
        )]));
    }
    let model = state.sidebar_model();
    assert_eq!(groups(&model)[0].shown, 7);

    state.apply_sidebar(SidebarAction::ShowMore(MoreKey::Project("p".to_owned())));

    let model = state.sidebar_model();
    assert_eq!(groups(&model)[0].shown, 10);
}

#[test]
fn a_settings_read_keeps_the_picks_an_earlier_write_did_not_confirm() {
    let mut state = loaded(api_settings());
    // Pick A, which a write carries first.
    state.apply_selection(SelectionAction::ToggleFavoriteAgent("a".to_owned()));
    let agents = state.settings().expect("settings").favorite_agents.clone();
    // Pick B while A is still in flight.
    state.apply_selection(SelectionAction::ToggleFavoriteModel {
        provider_id: "perf".to_owned(),
        model_id: "stream-1200cps".to_owned(),
    });
    let models = state.settings().expect("settings").favorite_models.clone();

    // A's answer holds the starred agents only.
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings(json!({
        "favoriteAgents": agents,
    })))));
    let after_a = state.settings().expect("settings");
    assert_eq!(after_a.favorite_agents, agents);
    assert_eq!(
        after_a.favorite_models[0].model_id, "stream-1200cps",
        "the pending pick survived the answer"
    );

    // Pick C after A's answer.
    state.apply_selection(SelectionAction::PickAgent("it-primary".to_owned()));
    let recent = state.settings().expect("settings").recent_agents.clone();
    assert_eq!(recent.first().map(String::as_str), Some("it-primary"));

    // The final answer echoes all three picks.
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings(json!({
        "favoriteAgents": agents,
        "favoriteModels": models,
        "recentAgents": recent,
    })))));
    let final_settings = state.settings().expect("settings");
    assert_eq!(final_settings.favorite_agents, agents);
    assert_eq!(final_settings.favorite_models[0].model_id, "stream-1200cps");
    assert_eq!(final_settings.recent_agents, recent);
    assert_eq!(
        state.settings_overlay,
        oc_client::SettingsPatch::default(),
        "confirmed picks stop being overlaid"
    );
}

#[test]
fn a_pending_collapse_survives_the_active_project_writes_answer() {
    let mut state = loaded(settings(json!({"projects": [{
        "id": "p", "path": "/p/app", "color": "#4F46E5", "icon": "rocket",
    }]})));
    // The project pick's write leaves first.
    state.apply_selection(SelectionAction::PickProject {
        id: "p".to_owned(),
        now: 1000,
    });
    // The collapse waits behind it.
    state.apply_sidebar(SidebarAction::ToggleProjectCollapsed("p".to_owned()));

    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings(json!({
        "activeProjectId": "p",
        "projects": [{
            "id": "p", "path": "/p/app", "lastOpenedAt": 1000,
            "color": "#4F46E5", "icon": "rocket",
        }],
    })))));

    let document = state.settings().expect("settings");
    assert_eq!(document.active_project_id.as_deref(), Some("p"));
    let project = &document.projects[0];
    assert_eq!(
        project.sidebar_collapsed(),
        Some(true),
        "the pending collapse survived the answer"
    );
    let object = serde_json::to_value(project).expect("json");
    assert_eq!(object["color"], "#4F46E5");
    assert_eq!(object["icon"], "rocket");
    assert!(state.settings_overlay.projects.is_some());
}

#[test]
fn a_refetch_page_failure_keeps_the_complete_rows_and_marks_them_incomplete() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));
    state.apply_sidebar(SidebarAction::Complete(api_sessions(
        "session-list-global-page-1",
    )));
    let rows = chats(&state.sidebar_model());
    assert!(rows > 0);

    state.apply_sidebar(SidebarAction::PageFailed("later".to_owned()));

    let model = state.sidebar_model();
    assert!(model.incomplete);
    assert!(!model.complete);
    assert_eq!(model.error, None);
    assert_eq!(chats(&model), rows, "the rows stay");
}

#[test]
fn a_failed_settings_refresh_keeps_the_document_and_shows_the_sidebar() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));
    let projects = groups(&state.sidebar_model()).len();

    state.apply_sidebar(SidebarAction::SettingsFailed("no settings".to_owned()));

    assert!(state.settings().is_some(), "the loaded settings stay");
    let model = state.sidebar_model();
    assert_eq!(model.error.as_deref(), Some("no settings"));
    assert_eq!(groups(&model).len(), projects, "the sidebar is not hidden");
}

#[test]
fn a_failed_home_refresh_keeps_the_sidebar_visible() {
    let mut state = loaded(api_settings());
    state.apply_sidebar(SidebarAction::HomeLoaded(api_home()));
    let projects = groups(&state.sidebar_model()).len();

    state.apply_sidebar(SidebarAction::HomeFailed("no home".to_owned()));

    assert!(state.home().is_some(), "the loaded home info stays");
    let model = state.sidebar_model();
    assert_eq!(model.error.as_deref(), Some("no home"));
    assert_eq!(groups(&model).len(), projects, "the sidebar is not hidden");
}
