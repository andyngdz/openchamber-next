//! Creating a session, with or without a Chat folder, and a draft's first prompt.

use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::types::ModelRef;
use oc_state::Action;

use super::{create, create_and_prompt};
use crate::live::fake_server_test::FakeServer;
use crate::live::protocol::{CreateTarget, Update};

const FOLDER: &str = "/chats/2026-11-10/session-00000000-0000-0000-0000-000000000000";
const CREATED: &str = r#"{"data":{"id":"ses_1","location":{"directory":"/p"}}}"#;
const PROMPTED: &str = r#"{"data":{"id":"msg_1"}}"#;

fn project() -> CreateTarget {
    CreateTarget::Project {
        directory: "/p".to_owned(),
    }
}

fn chat() -> CreateTarget {
    CreateTarget::Chat {
        folder: FOLDER.to_owned(),
    }
}

fn model() -> ModelRef {
    ModelRef {
        id: "m".to_owned(),
        provider_id: "p".to_owned(),
        variant: None,
    }
}

async fn drain(sender: mpsc::Sender<Update>, receiver: mpsc::Receiver<Update>) -> Vec<Update> {
    drop(sender);
    receiver.collect().await
}

#[tokio::test]
async fn creating_in_a_project_needs_no_folder() {
    let server = FakeServer::start(vec![("200 OK", CREATED)]);
    let (mut sender, receiver) = mpsc::channel(4);

    create(&server.client, &project(), None, None, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    assert!(
        matches!(&updates[..], [Update::Created { session, directory }] if session.id == "ses_1" && directory == "/p"),
        "{updates:?}"
    );
    assert_eq!(requests.len(), 1);
    let created = &requests[0];
    assert_eq!(created.path, "/api/session");
    assert_eq!(created.json()["location"]["directory"], "/p");
}

#[tokio::test]
async fn a_chat_session_creates_its_folder_first() {
    let server = FakeServer::start(vec![
        ("200 OK", r#"{"success":true,"path":"/chats"}"#),
        ("200 OK", CREATED),
    ]);
    let (mut sender, receiver) = mpsc::channel(4);

    create(
        &server.client,
        &chat(),
        Some("build"),
        Some(&model()),
        &mut sender,
    )
    .await
    .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    assert!(
        matches!(&updates[..], [Update::Created { directory, .. }] if directory == FOLDER),
        "{updates:?}"
    );
    let mkdir = &requests[0];
    assert_eq!(mkdir.path, "/api/fs/mkdir");
    assert_eq!(mkdir.json()["path"], FOLDER);
    let created = requests[1].json();
    assert_eq!(created["location"]["directory"], FOLDER);
    assert_eq!(created["agent"], "build");
    let created_model = &created["model"];
    assert_eq!(created_model["id"], "m");
    assert!(created_model.get("variant").is_none());
}

#[tokio::test]
async fn a_folder_failure_keeps_the_draft() {
    let server = FakeServer::start(vec![("500 Internal Server Error", "{}")]);
    let (mut sender, receiver) = mpsc::channel(4);

    create_and_prompt(&server.client, &chat(), "hello", None, None, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    drop(server.finish());

    let [Update::Apply(Action::PromptFailed(message))] = updates.as_slice() else {
        panic!("a composer error expected, got {updates:?}");
    };
    assert!(message.contains("chat folder"), "{message}");
}

#[tokio::test]
async fn a_chat_draft_creates_its_folder_session_and_first_prompt() {
    let server = FakeServer::start(vec![
        ("200 OK", r#"{"success":true}"#),
        ("200 OK", CREATED),
        ("200 OK", PROMPTED),
    ]);
    let (mut sender, receiver) = mpsc::channel(4);

    create_and_prompt(
        &server.client,
        &chat(),
        "hello",
        Some("build"),
        Some(&model()),
        &mut sender,
    )
    .await
    .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    assert!(
        matches!(&updates[..], [Update::CreatedAndSent { session, directory, text }] if session.id == "ses_1" && directory == FOLDER && text == "hello"),
        "{updates:?}"
    );
    let paths: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(
        paths,
        ["/api/fs/mkdir", "/api/session", "/api/session/ses_1/prompt"]
    );
}

#[tokio::test]
async fn a_failed_first_prompt_still_opens_the_created_session() {
    let server = FakeServer::start(vec![
        ("200 OK", r#"{"success":true}"#),
        ("200 OK", CREATED),
        ("500 Internal Server Error", "{}"),
    ]);
    let (mut sender, receiver) = mpsc::channel(4);

    create_and_prompt(&server.client, &chat(), "hello", None, None, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    drop(server.finish());

    let [
        Update::Created { session, .. },
        Update::Apply(Action::PromptFailed(message)),
    ] = updates.as_slice()
    else {
        panic!("an opened session and a composer error expected, got {updates:?}");
    };
    assert_eq!(session.id, "ses_1");
    assert!(message.contains("prompt"), "{message}");
}
