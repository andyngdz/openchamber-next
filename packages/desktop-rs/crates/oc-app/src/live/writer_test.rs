use crate::live::protocol::Update;
use futures::SinkExt;
use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::SettingsPatch;
use oc_state::SidebarAction;

use super::{COLLAPSE_DEBOUNCE, SETTINGS_DEBOUNCE, debounce_for, flush, merge, run_settings};
use crate::live::fake_server_test::FakeServer;
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

    flush(&server.client, &mut pending, &mut sender).await;
    assert!(pending.is_some(), "the choice is kept");
    flush(&server.client, &mut pending, &mut sender).await;
    assert!(pending.is_none(), "the retry sends it");
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
