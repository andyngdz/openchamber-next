//! The turn-control calls against a one-shot HTTP server, plus the bodies they send.

use serde_json::Value;

use crate::client::client_test::{Reply, Server, directory, fixture, head_end, ok_json, serve};
use crate::form::FormAnswer;
use crate::permission::PermissionReply;
use crate::prompt::Delivery;

const SESSION: &str = "ses_fixture";

fn json(name: &str) -> Value {
    serde_json::from_str(&fixture(name)).expect("fixture json")
}

fn body(request: &str) -> Value {
    let (_, body) = request
        .split_once(head_end().as_str())
        .expect("a body follows the head");
    serde_json::from_str(body).expect("json body")
}

#[test]
fn every_captured_interrupt_answer_decodes() {
    let mut names: Vec<String> = std::fs::read_dir(concat!(env!("OC_DESKTOP_FIXTURES"), "/api"))
        .expect("fixtures/api")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("interrupt-") && name.ends_with(".json"))
        .filter(|name| !name.ends_with(".request.json"))
        .collect();
    names.sort();
    assert!(names.len() >= 4, "{names:?}");
    for name in names {
        let captured = json(&format!("api/{name}"));
        let result: crate::types::InterruptResult =
            serde_json::from_value(captured.clone()).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!(
            result.interrupted,
            captured["interrupted"].as_bool().expect("flag"),
            "{name}"
        );
    }
}

fn no_content() -> Reply {
    Reply {
        status: "204 No Content",
        content_type: "application/json",
        body: String::new(),
    }
}

#[tokio::test]
async fn permissions_list_the_directory() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/permission-request-list.json"))]);

    let requests_seen = client.permissions(&directory()).await.expect("permissions");

    let captured = json("api/permission-request-list.json");
    assert_eq!(
        requests_seen.len(),
        captured["data"].as_array().expect("data").len()
    );
    let request = requests.recv().expect("request");
    assert!(request.starts_with(
        "GET /api/permission/request?location[directory]=%2Ftmp%2Fmy%20project HTTP/1.1"
    ));
    assert!(request.contains("x-opencode-directory: %2Ftmp%2Fmy%20project"));
}

#[tokio::test]
async fn a_permission_reply_posts_the_captured_decision() {
    let captured = json("api/permission-reply-once.request.json");
    let decision: PermissionReply =
        serde_json::from_value(captured["decision"].clone()).expect("decision");
    let Server {
        client, requests, ..
    } = serve(vec![no_content()]);

    client
        .reply_permission(&directory(), SESSION, "per_fixture", decision)
        .await
        .expect("reply");

    let request = requests.recv().expect("request");
    assert!(
        request.starts_with("POST /api/session/ses_fixture/permission/per_fixture/reply HTTP/1.1")
    );
    assert_eq!(body(&request), captured);
}

#[tokio::test]
async fn a_missing_permission_reply_is_not_found() {
    let Server {
        client,
        requests: _requests,
        ..
    } = serve(vec![Reply {
        status: "404 Not Found",
        content_type: "application/json",
        body: fixture("api/permission-reply-404.json"),
    }]);

    assert!(matches!(
        client
            .reply_permission(&directory(), SESSION, "per_fixture", PermissionReply::Once)
            .await,
        Err(crate::ClientError::NotFound { .. })
    ));
}

#[tokio::test]
async fn forms_list_the_directory_including_the_global_ones() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/form-list.json"))]);

    let forms = client.forms(&directory()).await.expect("forms");

    let captured = json("api/form-list.json");
    assert_eq!(
        forms.len(),
        captured["data"].as_array().expect("data").len()
    );
    assert!(forms.iter().any(|form| form.session_id == "global"));
    let request = requests.recv().expect("request");
    assert!(
        request.starts_with("GET /api/form?location[directory]=%2Ftmp%2Fmy%20project HTTP/1.1")
    );
}

#[tokio::test]
async fn a_form_reply_posts_the_captured_answer() {
    let captured = json("api/form-reply.request.json");
    let answer: FormAnswer = serde_json::from_value(captured["answer"].clone()).expect("answer");
    let Server {
        client, requests, ..
    } = serve(vec![no_content()]);

    client
        .reply_form(&directory(), SESSION, "frm_fixture", &answer)
        .await
        .expect("reply");

    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/session/ses_fixture/form/frm_fixture/reply HTTP/1.1"));
    assert_eq!(body(&request), captured);
}

#[tokio::test]
async fn a_form_cancel_deletes_and_a_missing_form_is_not_found() {
    let Server {
        client, requests, ..
    } = serve(vec![
        no_content(),
        Reply {
            status: "404 Not Found",
            content_type: "application/json",
            body: fixture("api/form-cancel-404.json"),
        },
    ]);
    let answer: FormAnswer = FormAnswer::new();

    client
        .cancel_form(&directory(), SESSION, "frm_fixture")
        .await
        .expect("cancel");
    assert!(matches!(
        client
            .reply_form(&directory(), SESSION, "frm_fixture", &answer)
            .await,
        Err(crate::ClientError::NotFound { .. })
    ));

    assert!(
        requests
            .recv()
            .expect("cancel request")
            .starts_with("DELETE /api/session/ses_fixture/form/frm_fixture HTTP/1.1")
    );
    assert!(
        requests
            .recv()
            .expect("reply request")
            .starts_with("POST /api/session/ses_fixture/form/frm_fixture/reply HTTP/1.1")
    );
}

#[tokio::test]
async fn interrupt_posts_without_a_body_and_decodes_the_answer() {
    let captured = json("api/interrupt-running.json");
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/interrupt-running.json"))]);

    let result = client
        .interrupt(&directory(), SESSION)
        .await
        .expect("interrupt");

    assert_eq!(
        result.interrupted,
        captured["interrupted"].as_bool().expect("flag")
    );
    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/session/ses_fixture/interrupt HTTP/1.1"));
    assert!(
        request.ends_with(head_end().as_str()),
        "no body: {request:?}"
    );
}

#[tokio::test]
async fn steer_prompt_posts_the_captured_delivery() {
    let captured = json("api/prompt-steer.request.json");
    let text = captured["text"].as_str().expect("text");
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/prompt-steer.json"))]);

    let accepted = client
        .steer_prompt(&directory(), SESSION, text)
        .await
        .expect("steer");

    assert_eq!(accepted.delivery, Some(Delivery::Steer));
    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/session/ses_fixture/prompt HTTP/1.1"));
    let sent = body(&request);
    assert_eq!(sent["text"].as_str(), Some(text));
    assert_eq!(sent["delivery"], captured["delivery"]);
    assert_eq!(sent.as_object().map(|object| object.len()), Some(3));
    assert!(
        sent["id"].as_str().is_some_and(|id| id.starts_with("msg_")),
        "{sent:?}"
    );
}
