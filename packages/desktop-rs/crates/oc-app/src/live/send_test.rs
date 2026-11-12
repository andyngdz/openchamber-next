use crate::live::protocol::Update;
use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::types::{ModelRef, SessionRecord};
use oc_state::{Action, SelectionAction, SessionId, State};

use super::{OpenSend, prompt};
use crate::live::fake_server_test::FakeServer;

const PROMPTED: &str = r#"{"data":{"id":"msg_1"}}"#;

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
        delivery: None,
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
    let model_switch = requests[2].json();
    let switched_model = &model_switch["model"];
    assert_eq!(switched_model["id"], "m");
    assert!(switched_model.get("variant").is_none());
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
        delivery: None,
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
        delivery: None,
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
        delivery: None,
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
            delivery: None,
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
