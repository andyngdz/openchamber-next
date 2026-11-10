use crate::live::protocol::Update;
use futures::SinkExt;
use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::SettingsPatch;
use oc_state::{SelectionAction, SidebarAction, State};

use super::{
    COLLAPSE_DEBOUNCE, SETTINGS_DEBOUNCE, debounce_for, merge, report, run_settings, write,
};
use crate::live::fake_server_test::{FakeServer, Gate};
use crate::live::protocol::SettingsCommand;

fn patch_agent(agent: &str) -> SettingsPatch {
    SettingsPatch {
        default_agent: Some(agent.to_owned()),
        ..SettingsPatch::default()
    }
}

fn patch_agents(agents: &[&str]) -> SettingsPatch {
    SettingsPatch {
        favorite_agents: Some(agents.iter().map(|agent| (*agent).to_owned()).collect()),
        ..SettingsPatch::default()
    }
}

fn patch_collapse() -> SettingsPatch {
    let project: oc_client::Project = serde_json::from_value(
        serde_json::json!({"id": "p", "path": "/p", "sidebarCollapsed": true}),
    )
    .expect("project");
    SettingsPatch {
        projects: Some(vec![project]),
        ..SettingsPatch::default()
    }
}

fn loaded_state() -> State {
    let mut state = State::new("/home/u");
    let settings = serde_json::from_value(serde_json::json!({"projects": []})).expect("settings");
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings)));
    state
}

fn apply_answer(state: &mut State, update: Update) {
    if let Update::Sidebar(action) = update {
        state.apply_sidebar(action);
    }
}

async fn wait_accepted(server: &FakeServer, count: usize) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while server.accepted() < count {
        assert!(
            std::time::Instant::now() < deadline,
            "the write did not reach the server"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

#[test]
fn a_later_key_wins_and_an_earlier_one_survives() {
    let merged = merge(Some(patch_agents(&["a"])), patch_agent("build"));

    assert_eq!(merged.default_agent.as_deref(), Some("build"));
    assert_eq!(
        merged.favorite_agents.as_deref(),
        Some(["a".to_owned()].as_slice())
    );
    assert_eq!(
        merge(None, patch_agent("build")).default_agent.as_deref(),
        Some("build")
    );
}

#[test]
fn a_collapse_waits_longer_than_other_writes() {
    assert_eq!(debounce_for(&patch_collapse()), COLLAPSE_DEBOUNCE);
    assert_eq!(debounce_for(&patch_agent("build")), SETTINGS_DEBOUNCE);
    let mut mixed = patch_collapse();
    mixed.active_project_id = Some("p".to_owned());
    assert_eq!(debounce_for(&mixed), SETTINGS_DEBOUNCE);
}

#[tokio::test]
async fn queued_writes_merge_into_one_put_that_replaces_the_settings() {
    let server = FakeServer::start(vec![(
        "200 OK",
        r#"{"favoriteAgents":["b"],"projects":[]}"#,
    )]);
    let (mut commands, command_receiver) = mpsc::channel(4);
    let (sender, updates) = mpsc::channel(8);
    commands
        .send(SettingsCommand::Write(Box::new(patch_agents(&["a"]))))
        .await
        .expect("send");
    commands
        .send(SettingsCommand::Write(Box::new(patch_agents(&["b"]))))
        .await
        .expect("send");
    drop(commands);

    let ((), updates) = tokio::join!(
        run_settings(server.client.clone(), command_receiver, sender),
        updates.collect::<Vec<Update>>()
    );
    let requests = server.finish();

    assert_eq!(requests.len(), 1, "one debounced write");
    assert_eq!(requests[0].method, "PUT");
    assert_eq!(
        requests[0].json()["favoriteAgents"],
        serde_json::json!(["b"])
    );
    let [Update::Sidebar(SidebarAction::SettingsLoaded(settings))] = updates.as_slice() else {
        panic!("the answer should replace the settings, got {updates:?}");
    };
    assert_eq!(
        settings.favorite_agents,
        vec!["b".to_owned()],
        "the server's answer is the new settings"
    );
}

#[tokio::test]
async fn a_failed_write_keeps_the_patch_for_the_next_one() {
    let server = FakeServer::start(vec![
        ("500 Internal Server Error", "{}"),
        ("200 OK", r#"{"favoriteAgents":["a"],"projects":[]}"#),
    ]);
    let (mut sender, receiver) = mpsc::channel(8);
    let mut pending = Some(patch_agents(&["a"]));

    assert!(write(&server.client, &mut pending).await.is_none());
    assert!(pending.is_some(), "the choice is kept");
    let answer = write(&server.client, &mut pending).await;
    assert!(pending.is_none(), "the retry sends it");
    report(&mut sender, answer).await;
    drop(sender);
    let updates: Vec<Update> = receiver.collect().await;
    let requests = server.finish();

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    assert!(
        matches!(
            updates.as_slice(),
            [Update::Sidebar(SidebarAction::SettingsLoaded(_))]
        ),
        "{updates:?}"
    );
}

#[tokio::test]
async fn a_full_update_channel_does_not_hold_the_flush_answer() {
    let server = FakeServer::start(vec![("200 OK", r#"{"favoriteAgents":["a"]}"#)]);
    let (mut commands, command_receiver) = mpsc::channel(4);
    let (sender, mut updates) = mpsc::channel(1);
    // Fill the channel as a stopped reader would.
    sender
        .clone()
        .try_send(Update::Authenticated)
        .expect("fill");
    let task = tokio::spawn(run_settings(
        server.client.clone(),
        command_receiver,
        sender,
    ));

    commands
        .send(SettingsCommand::Write(Box::new(patch_agents(&["a"]))))
        .await
        .expect("write");
    let (done, finished) = std::sync::mpsc::channel();
    commands
        .send(SettingsCommand::Flush(done))
        .await
        .expect("flush");

    let answered = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            match finished.try_recv() {
                Ok(()) => return,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Err(error) => panic!("the flush channel ended: {error}"),
            }
        }
    })
    .await;
    assert!(
        answered.is_ok(),
        "the flush answers before forwarding the settings"
    );
    drop(updates.next().await);
    task.abort();
    drop(server.finish());
}

#[tokio::test]
async fn a_pick_queued_during_a_write_reaches_the_next_put_and_the_read_keeps_it() {
    let gate = Gate::closed();
    let server = FakeServer::start_gated(vec![
        ("200 OK", r#"{"favoriteAgents":["a"]}"#, gate.clone()),
        (
            "200 OK",
            r#"{"favoriteAgents":["a"],"favoriteModels":[{"providerID":"p","modelID":"m"}],"recentAgents":["x"]}"#,
            Gate::passing(),
        ),
    ]);
    let (mut commands, command_receiver) = mpsc::channel(8);
    let (sender, updates) = mpsc::channel(8);
    let mut state = loaded_state();
    let writer = tokio::spawn(run_settings(
        server.client.clone(),
        command_receiver,
        sender,
    ));

    // Pick A and let its write leave while the server holds the answer.
    state.apply_selection(SelectionAction::ToggleFavoriteAgent("a".to_owned()));
    let a = state.take_settings_patch().expect("A");
    commands
        .send(SettingsCommand::Write(Box::new(a)))
        .await
        .expect("A");
    wait_accepted(&server, 1).await;

    // Pick B while A is in flight, and C before the answer lands.
    state.apply_selection(SelectionAction::ToggleFavoriteModel {
        provider_id: "p".to_owned(),
        model_id: "m".to_owned(),
    });
    let b = state.take_settings_patch().expect("B");
    commands
        .send(SettingsCommand::Write(Box::new(b)))
        .await
        .expect("B");
    gate.open();
    state.apply_selection(SelectionAction::PickAgent("x".to_owned()));
    let c = state.take_settings_patch().expect("C");
    commands
        .send(SettingsCommand::Write(Box::new(c)))
        .await
        .expect("C");
    drop(commands);

    for update in updates.collect::<Vec<Update>>().await {
        apply_answer(&mut state, update);
    }
    writer.await.expect("writer");
    let requests = server.finish();

    assert_eq!(requests.len(), 2, "one write per settled batch");
    let final_put = requests[1].json();
    assert_eq!(final_put["favoriteModels"][0]["modelID"], "m");
    assert_eq!(final_put["recentAgents"], serde_json::json!(["x"]));
    let document = state.settings().expect("settings");
    assert_eq!(document.favorite_agents, ["a".to_owned()]);
    assert_eq!(document.favorite_models.len(), 1);
    assert_eq!(document.recent_agents, ["x".to_owned()]);
}
