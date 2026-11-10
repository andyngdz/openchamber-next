use crate::live::protocol::Update;
use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::types::{ModelRef, SessionRecord};
use oc_state::{Action, SelectionAction, SessionId, State};

use super::{OpenSend, create, create_and_prompt, prompt};
use crate::live::fake_server_test::FakeServer;
use crate::live::protocol::CreateTarget;

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

fn model(variant: Option<&str>) -> ModelRef {
    ModelRef {
        id: "m".to_owned(),
        provider_id: "p".to_owned(),
        variant: variant.map(str::to_owned),
    }
}

fn named(id: &str) -> ModelRef {
    ModelRef {
        id: id.to_owned(),
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
    assert_eq!(requests[0].path, "/api/session");
    assert_eq!(requests[0].json()["location"]["directory"], "/p");
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
        Some(&model(None)),
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
    assert_eq!(requests[0].path, "/api/fs/mkdir");
    assert_eq!(requests[0].json()["path"], FOLDER);
    assert_eq!(requests[1].json()["location"]["directory"], FOLDER);
    assert_eq!(requests[1].json()["agent"], "build");
    assert_eq!(requests[1].json()["model"]["id"], "m");
    assert!(requests[1].json()["model"].get("variant").is_none());
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
        Some(&model(None)),
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

#[tokio::test]
async fn an_open_session_switches_its_agent_and_model_before_the_prompt() {
    let record = r#"{"data":{"id":"ses_a","agent":"build","model":{"id":"m","providerID":"p","variant":"high"}}}"#;
    let server = FakeServer::start(vec![
        ("200 OK", record),
        ("204 No Content", ""),
        ("204 No Content", ""),
        ("200 OK", PROMPTED),
    ]);
    let (mut sender, receiver) = mpsc::channel(4);
    let session = SessionId::from("ses_a");
    let next = model(None);
    let previous = model(Some("high"));
    let open = OpenSend {
        session: &session,
        directory: "/p",
        text: "hello",
        agent: Some("plan"),
        model: Some(&next),
        record_model: Some(&previous),
    };

    prompt(&server.client, open, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    let switched: Vec<&SelectionAction> = updates
        .iter()
        .filter_map(|update| match update {
            Update::Selection(action) => Some(action),
            _ => None,
        })
        .collect();
    assert!(
        matches!(
            switched.as_slice(),
            [
                SelectionAction::AgentSwitched { agent, .. },
                SelectionAction::ModelSwitched { model, .. }
            ] if agent == "plan" && model.id == "m"
        ),
        "{updates:?}"
    );
    assert!(
        matches!(updates.last(), Some(Update::Sent { text }) if text == "hello"),
        "{updates:?}"
    );
    let paths: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "/api/session/ses_a",
            "/api/session/ses_a/agent",
            "/api/session/ses_a/model",
            "/api/session/ses_a/prompt"
        ]
    );
    assert_eq!(requests[1].json()["agent"], "plan");
    assert_eq!(requests[2].json()["model"]["id"], "m");
    assert!(requests[2].json()["model"].get("variant").is_none());
}

#[tokio::test]
async fn a_matching_agent_and_model_only_prompt() {
    let record = r#"{"data":{"id":"ses_a","agent":"build","model":{"id":"m","providerID":"p"}}}"#;
    let server = FakeServer::start(vec![("200 OK", record), ("200 OK", PROMPTED)]);
    let (mut sender, receiver) = mpsc::channel(4);
    let session = SessionId::from("ses_a");
    let same = model(None);

    let open = OpenSend {
        session: &session,
        directory: "/p",
        text: "hello",
        agent: Some("build"),
        model: Some(&same),
        record_model: Some(&same),
    };
    prompt(&server.client, open, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    assert!(matches!(&updates[..], [Update::Sent { .. }]), "{updates:?}");
    assert_eq!(requests.len(), 2);
}

#[tokio::test]
async fn a_model_switch_failure_never_prompts() {
    let server = FakeServer::start(vec![("500 Internal Server Error", "{}")]);
    let (mut sender, receiver) = mpsc::channel(4);
    let session = SessionId::from("ses_a");
    let next = model(None);
    let previous = model(Some("high"));

    let open = OpenSend {
        session: &session,
        directory: "/p",
        text: "hello",
        agent: None,
        model: Some(&next),
        record_model: Some(&previous),
    };
    prompt(&server.client, open, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    let [Update::Apply(Action::PromptFailed(message))] = updates.as_slice() else {
        panic!("a composer error expected, got {updates:?}");
    };
    assert!(message.contains("model"), "{message}");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/api/session/ses_a/model");
}

#[tokio::test]
async fn switching_only_the_variant_sends_the_model_again() {
    let record = r#"{"data":{"id":"ses_a","agent":"build","model":{"id":"m","providerID":"p","variant":"high"}}}"#;
    let server = FakeServer::start(vec![
        ("200 OK", record),
        ("204 No Content", ""),
        ("200 OK", PROMPTED),
    ]);
    let (mut sender, receiver) = mpsc::channel(4);
    let session = SessionId::from("ses_a");
    let next = model(Some("low"));
    let previous = model(Some("high"));

    let open = OpenSend {
        session: &session,
        directory: "/p",
        text: "hello",
        agent: Some("build"),
        model: Some(&next),
        record_model: Some(&previous),
    };
    prompt(&server.client, open, &mut sender)
        .await
        .expect("window open");
    let updates = drain(sender, receiver).await;
    let requests = server.finish();

    assert!(
        matches!(updates.last(), Some(Update::Sent { .. })),
        "{updates:?}"
    );
    assert_eq!(requests[1].json()["model"]["variant"], "low");
}

#[tokio::test]
async fn a_model_switch_and_back_sends_the_model_both_times() {
    let record = r#"{"data":{"id":"ses_a","agent":"build","model":{"id":"a","providerID":"p"}}}"#;
    let server = FakeServer::start(vec![
        ("200 OK", record),
        ("204 No Content", ""),
        ("200 OK", PROMPTED),
        ("200 OK", record),
        ("204 No Content", ""),
        ("200 OK", PROMPTED),
    ]);
    let mut state = State::new("/p");
    let session = SessionId::from("ses_a");
    let first = named("a");
    let second = named("b");
    state.apply_selection(SelectionAction::SessionRecordLoaded {
        session: session.clone(),
        record: SessionRecord {
            id: "ses_a".to_owned(),
            agent: Some("build".to_owned()),
            model: Some(first.clone()),
        },
    });

    for target in [&second, &first] {
        let record_model = state
            .session_record(&session)
            .and_then(|record| record.model.clone());
        let (mut sender, receiver) = mpsc::channel(8);
        let open = OpenSend {
            session: &session,
            directory: "/p",
            text: "hello",
            agent: Some("build"),
            model: Some(target),
            record_model: record_model.as_ref(),
        };
        prompt(&server.client, open, &mut sender)
            .await
            .expect("window open");
        drop(sender);
        for update in receiver.collect::<Vec<Update>>().await {
            if let Update::Selection(action) = update {
                state.apply_selection(action);
            }
        }
    }
    let requests = server.finish();

    let model_switches: Vec<String> = requests
        .iter()
        .filter(|request| request.path == "/api/session/ses_a/model")
        .map(|request| {
            request.json()["model"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(
        model_switches,
        vec!["b".to_owned(), "a".to_owned()],
        "both switches sent a model"
    );
}
