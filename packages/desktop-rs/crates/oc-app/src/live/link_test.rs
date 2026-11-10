//! The live link's settings writes: a new connection resends the held overlay as one write,
//! and a write the bounded queue cannot take is abandoned.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures::channel::mpsc;
use gpui_kit::Task;
use oc_client::SettingsPatch;
use tokio::runtime::Handle;

use super::fake_server_test::FakeServer;
use super::{ServerLink, SettingsCommand, WriteOutcome, deliver};
use crate::server::ServerProcess;

/// A patch with two keys, so a resend must carry the whole overlay, not one field.
fn overlay() -> SettingsPatch {
    SettingsPatch {
        default_agent: Some("build".to_owned()),
        favorite_agents: Some(vec!["a".to_owned()]),
        ..SettingsPatch::default()
    }
}

#[test]
fn a_write_with_no_connection_is_held_for_the_next_one() {
    // `None` is what `write_settings` passes while disconnected; the overlay keeps the
    // write until `connect` resends it.
    assert_eq!(deliver(None, overlay()), WriteOutcome::Held);
}

#[test]
fn a_connected_write_is_one_whole_patch() {
    let (mut settings, mut queued) = mpsc::channel(4);
    let patch = overlay();

    assert_eq!(
        deliver(Some(&mut settings), patch.clone()),
        WriteOutcome::Queued
    );

    // This is exactly what `connect` hands the writer: the whole overlay in one write.
    match queued.try_recv().expect("one write") {
        SettingsCommand::Write(written) => assert_eq!(*written, patch),
        other => panic!("expected a settings write, got {other:?}"),
    }
    assert!(
        queued.try_recv().is_err(),
        "the overlay is a single write, not one per key"
    );
}

#[test]
fn a_full_queue_drops_the_write() {
    let (mut settings, _queued) = mpsc::channel(1);
    // Keep the receiver alive: this is a full queue, not a closed channel.
    let mut outcome = WriteOutcome::Queued;
    for _ in 0..32 {
        outcome = deliver(Some(&mut settings), overlay());
        if outcome == WriteOutcome::Dropped {
            break;
        }
    }
    assert_eq!(
        outcome,
        WriteOutcome::Dropped,
        "a bounded queue eventually abandons a write"
    );
}

/// A new connection resends the whole overlay as its first write. The fake server scripts
/// every connection with the same body: it answers both the auth check and the settings PUT,
/// so the test does not depend on which request opens a connection first.
#[tokio::test]
async fn a_new_connection_resends_the_whole_overlay_in_one_write() {
    const REPLY: &str = r#"{"authenticated":true,"projects":[]}"#;
    let server = FakeServer::start(vec![
        ("200 OK", REPLY),
        ("200 OK", REPLY),
        ("200 OK", REPLY),
        ("200 OK", REPLY),
    ]);
    let process = ServerProcess::spawn(PathBuf::from("/tmp")).expect("server thread");
    let mut link = ServerLink::new(process, Handle::current().clone());
    let patch = overlay();

    // Dropping the updates receiver ends the session task after its auth check, so the
    // settings writer is the only task that keeps talking to the server.
    link.connect(server.port(), Some(patch.clone()), |_updates| {
        Task::ready(())
    })
    .expect("connect");

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if server
            .requests()
            .iter()
            .any(|request| request.method == "PUT")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the overlay was not resent as a write"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    drop(link);
    let requests = server.finish();

    let puts: Vec<_> = requests
        .iter()
        .filter(|request| request.method == "PUT")
        .collect();
    assert_eq!(
        puts.len(),
        1,
        "the whole overlay is one write: {requests:?}"
    );
    let body = puts[0].json();
    assert_eq!(body["defaultAgent"], serde_json::json!("build"));
    assert_eq!(body["favoriteAgents"], serde_json::json!(["a"]));
}
