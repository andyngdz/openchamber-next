use crate::live::protocol::Update;
use futures::StreamExt;
use futures::channel::mpsc;
use oc_state::SelectionAction;

use super::load;
use crate::live::fake_server_test::FakeServer;

const DIRECTORY: &str = "/tmp/project";

fn catalog_replies() -> Vec<(&'static str, &'static str)> {
    vec![
        ("200 OK", "[]"),
        (
            "200 OK",
            r#"{"data":[{"id":"p","name":"P","activation":"enabled"}]}"#,
        ),
        (
            "200 OK",
            r#"{"data":[{"id":"m","providerID":"p","name":"M","enabled":true}]}"#,
        ),
        (
            "200 OK",
            r#"{"data":[{"id":"build","name":"Build","mode":"primary"}]}"#,
        ),
    ]
}

async fn run(replies: Vec<(&'static str, &'static str)>) -> (Vec<Update>, Vec<String>) {
    let server = FakeServer::start(replies);
    let (mut sender, receiver) = mpsc::channel(4);
    load(&server.client, DIRECTORY, &mut sender)
        .await
        .expect("window open");
    drop(sender);
    let updates: Vec<Update> = receiver.collect().await;
    let paths = server.finish().into_iter().map(|r| r.path).collect();
    (updates, paths)
}

#[tokio::test]
async fn all_four_answers_become_one_catalog() {
    let (updates, paths) = run(catalog_replies()).await;

    let [Update::Selection(SelectionAction::CatalogLoaded { directory, catalog })] =
        updates.as_slice()
    else {
        panic!("a catalog expected, got {updates:?}");
    };
    assert_eq!(directory, DIRECTORY);
    assert_eq!(catalog.providers().len(), 1);
    assert_eq!(catalog.providers()[0].models[0].id, "m");
    assert_eq!(catalog.picker_agents()[0].id, "build");
    assert_eq!(paths.len(), 4);
}

#[tokio::test]
async fn a_failed_answer_is_a_catalog_error_with_its_directory() {
    let mut replies = catalog_replies();
    replies[1] = ("500 Internal Server Error", r#"{"error":"boom"}"#);

    let (updates, _) = run(replies).await;

    let [Update::Selection(SelectionAction::CatalogFailed { directory, message })] =
        updates.as_slice()
    else {
        panic!("a catalog failure expected, got {updates:?}");
    };
    assert_eq!(directory, DIRECTORY);
    assert!(message.contains("500"), "{message}");
}
