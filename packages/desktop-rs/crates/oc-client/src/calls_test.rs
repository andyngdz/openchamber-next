//! The new client calls against a one-shot HTTP server, plus the request shapes they send.

use serde_json::Value;

use crate::Directory;
use crate::client::client_test::{Reply, Server, directory, fixture, head_end, ok_json, serve};
use crate::uri_component::encode_uri_component;

fn json(text: &str) -> Value {
    serde_json::from_str(text).expect("json")
}

fn no_directory_header(request: &str) -> bool {
    !request
        .to_ascii_lowercase()
        .contains("x-opencode-directory")
}

fn body(request: &str) -> Value {
    let (_, body) = request
        .split_once(head_end().as_str())
        .expect("a body follows the head");
    json(body)
}

#[tokio::test]
async fn home_is_unscoped() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/fs-home.json"))]);

    let home = client.home().await.expect("home");

    assert!(home.chats_root.ends_with("/oc-data/chats"));
    let request = requests.recv().expect("request");
    assert!(request.starts_with("GET /api/fs/home HTTP/1.1"));
    assert!(no_directory_header(&request));
}

#[tokio::test]
async fn settings_reads_the_desktop_surface_without_a_directory() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/settings.json"))]);

    let settings = client.settings().await.expect("settings");

    assert_eq!(settings.projects.len(), 2);
    let request = requests.recv().expect("request");
    assert!(request.starts_with("GET /api/config/settings?surface=desktop HTTP/1.1"));
    assert!(no_directory_header(&request));
}

#[tokio::test]
async fn a_settings_write_is_a_partial_put_answered_with_the_whole_document() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/settings-update.json"))]);
    let patch: crate::SettingsPatch =
        serde_json::from_str(&fixture("api/settings-update.request.json")).expect("patch");

    let settings = client.update_settings(&patch).await.expect("write");

    assert_ne!(
        settings.active_project_id,
        json(&fixture("api/settings.json"))["activeProjectId"]
            .as_str()
            .map(str::to_owned)
    );
    let request = requests.recv().expect("request");
    assert!(request.starts_with("PUT /api/config/settings?surface=desktop HTTP/1.1"));
    assert!(no_directory_header(&request));
    assert_eq!(
        body(&request),
        json(&fixture("api/settings-update.request.json")),
        "only the keys the patch names are sent"
    );
}

#[tokio::test]
async fn the_catalog_calls_carry_the_directory_header() {
    let Server {
        client, requests, ..
    } = serve(vec![
        ok_json(fixture("api/providers.json")),
        ok_json(fixture("api/models.json")),
        ok_json(fixture("api/agents.json")),
        ok_json(fixture("api/config.json")),
    ]);
    let source = directory();

    let providers = client.providers(&source).await.expect("providers");
    let models = client.models(&source).await.expect("models");
    let agents = client.agents(&source).await.expect("agents");
    let config = client.config(&source).await.expect("config");

    assert_eq!(providers.len(), 3);
    assert!(models.len() >= 30);
    assert_eq!(agents.len(), 11);
    assert_eq!(config.default_agent.as_deref(), Some("capture-secondary"));
    let paths = [
        "GET /api/provider HTTP/1.1",
        "GET /api/model?types=chat&types=image HTTP/1.1",
        "GET /api/agent HTTP/1.1",
        "GET /api/config HTTP/1.1",
    ];
    for path in paths {
        let request = requests.recv().expect("request");
        assert!(request.starts_with(path), "{request}");
        assert!(request.contains("x-opencode-directory: %2Ftmp%2Fmy%20project"));
    }
}

#[tokio::test]
async fn mkdir_posts_the_folder_without_a_directory() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/fs-mkdir.json"))]);
    let folder = Directory::new(
        json(&fixture("api/fs-mkdir.request.json"))["path"]
            .as_str()
            .expect("path"),
    );

    let created = client.mkdir(&folder).await.expect("mkdir");

    assert!(created.success);
    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/fs/mkdir HTTP/1.1"));
    assert!(no_directory_header(&request));
    assert_eq!(body(&request), json(&fixture("api/fs-mkdir.request.json")));
}

#[tokio::test]
async fn create_session_sends_the_agent_and_model_of_the_pick() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture(
        "api/session-create-agent-model.json",
    ))]);
    let captured = json(&fixture("api/session-create-agent-model.request.json"));
    let project = Directory::new(
        captured["location"]["directory"]
            .as_str()
            .expect("directory"),
    );
    let model = crate::types::ModelRef {
        id: "alt-thinking".to_owned(),
        provider_id: "perf-second".to_owned(),
        variant: Some("medium".to_owned()),
    };

    let created = client
        .create_session(&project, Some("capture-primary"), Some(&model))
        .await
        .expect("create");

    assert!(created.id.starts_with("ses_"));
    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/session HTTP/1.1"));
    assert_eq!(body(&request), captured);
}

#[tokio::test]
async fn the_model_and_agent_switches_accept_a_204() {
    let Server {
        client, requests, ..
    } = serve(vec![
        Reply {
            status: "204 No Content",
            content_type: "application/json",
            body: String::new(),
        },
        Reply {
            status: "204 No Content",
            content_type: "application/json",
            body: String::new(),
        },
    ]);
    let model = crate::types::ModelRef {
        id: "stream-600cps".to_owned(),
        provider_id: "perf".to_owned(),
        variant: Some("high".to_owned()),
    };

    client
        .switch_model(&directory(), "ses_1", &model)
        .await
        .expect("model");
    client
        .switch_agent(&directory(), "ses_1", "build")
        .await
        .expect("agent");

    let model_request = requests.recv().expect("request");
    assert!(model_request.starts_with("POST /api/session/ses_1/model HTTP/1.1"));
    assert_eq!(
        body(&model_request),
        json(&fixture("api/switch-model-variant.request.json"))
    );
    let agent_request = requests.recv().expect("request");
    assert!(agent_request.starts_with("POST /api/session/ses_1/agent HTTP/1.1"));
    assert_eq!(
        body(&agent_request),
        json(&fixture("api/switch-agent.request.json"))
    );
}

#[tokio::test]
async fn the_global_session_page_has_no_directory_query_or_header() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture(
        "api/session-list-global-page-1.json",
    ))]);

    let page = client.session_page(500, None).await.expect("page");

    assert_eq!(page.data.len(), 2);
    assert!(page.cursor.next.is_some());
    let request = requests.recv().expect("request");
    assert!(request.starts_with("GET /api/session?limit=500 HTTP/1.1"));
    assert!(!request.contains("directory="));
    assert!(no_directory_header(&request));
}

#[tokio::test]
async fn a_global_page_sends_its_cursor_encoded() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture(
        "api/session-list-global-page-2.json",
    ))]);
    let cursor = "next/+=";

    client.session_page(500, Some(cursor)).await.expect("page");

    let request = requests.recv().expect("request");
    assert!(request.starts_with(&format!(
        "GET /api/session?limit=500&cursor={} HTTP/1.1",
        encode_uri_component(cursor)
    )));
}

#[tokio::test]
async fn the_global_walk_requests_pages_until_a_page_adds_nothing() {
    let Server {
        client, requests, ..
    } = serve(vec![
        ok_json(fixture("api/session-list-global-page-1.json")),
        ok_json(fixture("api/session-list-global-page-2.json")),
        ok_json(fixture("api/session-list-global-page-2.json")),
    ]);

    let sessions = client.list_global_sessions().await.expect("walk");

    let first = json(&fixture("api/session-list-global-page-1.json"));
    let second = json(&fixture("api/session-list-global-page-2.json"));
    let mut expected: Vec<&str> = Vec::new();
    for page in [&first, &second] {
        for session in page["data"].as_array().expect("page data") {
            expected.push(session["id"].as_str().expect("session id"));
        }
    }
    assert_eq!(sessions.len(), expected.len());
    let ids: Vec<&str> = sessions.iter().map(|session| session.id.as_str()).collect();
    assert_eq!(ids, expected);
    assert_eq!(requests.try_iter().count(), 3);
}
