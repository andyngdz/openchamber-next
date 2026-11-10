use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use futures::StreamExt;

use super::Client;
use crate::ClientError;
use crate::Directory;
use crate::event::EventData;

const FIXTURES: &str = env!("OC_DESKTOP_FIXTURES");
const DIRECTORY: &str = "/tmp/my project";
const ENCODED_DIRECTORY: &str = "%2Ftmp%2Fmy%20project";
const CRLF: &str = "\r\n";
const LF: &str = "\n";

pub(crate) fn directory() -> Directory {
    Directory::new(DIRECTORY)
}

pub(crate) struct Reply {
    pub(crate) status: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) body: String,
}

pub(crate) fn ok_json(body: String) -> Reply {
    Reply {
        status: "200 OK",
        content_type: "application/json",
        body,
    }
}

pub(crate) fn fixture(path: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURES}/{path}")).expect("fixture")
}

pub(crate) struct Server {
    pub(crate) client: Client,
    pub(crate) requests: Receiver<String>,
    // Held so the worker is owned by the test; it ends after the last reply.
    pub(crate) _worker: JoinHandle<()>,
}

/// One-shot HTTP server on a free port: answers each connection with the next reply and
/// sends the raw request (head and body) of each on `requests`.
pub(crate) fn serve(replies: Vec<Reply>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (sent, requests) = mpsc::channel();
    let worker = thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = listener.accept().expect("accept");
            sent.send(read_request(&mut stream))
                .expect("record request");
            let head = [
                format!("HTTP/1.1 {}", reply.status),
                format!("content-type: {}", reply.content_type),
                "connection: close".to_owned(),
                String::new(),
                String::new(),
            ]
            .join(CRLF);
            stream.write_all(head.as_bytes()).expect("write head");
            stream.write_all(reply.body.as_bytes()).expect("write body");
        }
    });
    let client = Client::new(port).expect("client");
    Server {
        client,
        requests,
        _worker: worker,
    }
}

pub(crate) fn head_end() -> String {
    CRLF.repeat(2)
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let read = stream.read(&mut chunk).unwrap_or(0);
        if read == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&data).into_owned();
        if let Some((head, body)) = text.split_once(head_end().as_str()) {
            let length = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length: ")?
                        .parse()
                        .ok()
                })
                .unwrap_or(0usize);
            if body.len() >= length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&data).into_owned()
}

#[tokio::test]
async fn auth_check_accepts_the_captured_status() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/auth-status.json"))]);
    client.check_auth().await.expect("authenticated");
    assert!(
        requests
            .recv()
            .expect("request")
            .starts_with("GET /auth/session HTTP/1.1")
    );
}

#[tokio::test]
async fn auth_check_rejects_authenticated_false() {
    let server = serve(vec![ok_json(r#"{"authenticated":false}"#.into())]);
    let client = &server.client;
    assert!(matches!(client.check_auth().await, Err(ClientError::Auth)));
}

#[tokio::test]
async fn list_sessions_sends_the_directory_and_decodes() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/session-list-after-turn.json"))]);
    let sessions = client.list_sessions(&directory()).await.expect("list");
    assert_eq!(sessions.len(), 1);
    let request = requests.recv().expect("request");
    assert!(request.starts_with(&format!(
        "GET /api/session?limit=50&directory={ENCODED_DIRECTORY} HTTP/1.1"
    )));
    assert!(request.contains(&format!("x-opencode-directory: {ENCODED_DIRECTORY}")));
}

#[tokio::test]
async fn create_session_posts_the_location() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/session-create.json"))]);
    let session = client
        .create_session(&directory(), None, None)
        .await
        .expect("create");
    assert!(session.id.starts_with("ses_"));
    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/session HTTP/1.1"));
    assert!(request.ends_with(r#"{"location":{"directory":"/tmp/my project"}}"#));
}

#[tokio::test]
async fn list_messages_decodes_a_turn() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/messages-agent-turn.json"))]);
    let messages = client
        .list_messages(&directory(), "ses_1")
        .await
        .expect("messages");
    assert!(messages.len() > 21);
    assert!(
        requests
            .recv()
            .expect("request")
            .starts_with("GET /api/session/ses_1/message HTTP/1.1")
    );
}

#[tokio::test]
async fn send_prompt_posts_a_minted_id_and_the_text() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/prompt-text.json"))]);
    let accepted = client
        .send_prompt(&directory(), "ses_1", "hello")
        .await
        .expect("prompt");
    assert!(accepted.id.starts_with("msg_"));
    let request = requests.recv().expect("request");
    assert!(request.starts_with("POST /api/session/ses_1/prompt HTTP/1.1"));
    let body: serde_json::Value =
        serde_json::from_str(request.split_once(head_end().as_str()).expect("body").1)
            .expect("json");
    assert_eq!(body["text"], "hello");
    assert!(body["id"].as_str().is_some_and(|id| id.starts_with("msg_")));
    assert_eq!(body.as_object().map(|o| o.len()), Some(2));
}

#[tokio::test]
async fn non_2xx_is_a_status_error_with_the_body() {
    let server = serve(vec![Reply {
        status: "500 Internal Server Error",
        content_type: "text/plain",
        body: "boom".into(),
    }]);
    let client = &server.client;
    match client.list_sessions(&directory()).await {
        Err(ClientError::Status { status, body }) => {
            assert_eq!(status, 500);
            assert_eq!(body, "boom");
        }
        other => panic!("expected a status error, got {other:?}"),
    }
}

#[tokio::test]
async fn undecodable_body_is_a_decode_error_not_an_empty_list() {
    let server = serve(vec![ok_json("<html>not the api</html>".into())]);
    let client = &server.client;
    assert!(matches!(
        client.list_sessions(&directory()).await,
        Err(ClientError::Decode(_))
    ));
}

#[tokio::test]
async fn refused_connection_is_a_transport_error() {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let client = Client::new(port).expect("client");
    assert!(matches!(
        client.list_sessions(&directory()).await,
        Err(ClientError::Transport(_))
    ));
}

#[tokio::test]
async fn silent_server_hits_the_call_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let client = Client::with_base_url(
        format!("http://127.0.0.1:{port}"),
        Duration::from_millis(200),
    )
    .expect("client");
    // The listener accepts at the kernel level but never answers.
    assert!(matches!(
        client.check_auth().await,
        Err(ClientError::Transport(_))
    ));
    drop(listener);
}

#[tokio::test]
async fn event_stream_decodes_frames_and_skips_heartbeats() {
    let frames: Vec<serde_json::Value> =
        serde_json::from_str(&fixture("events/session-lifecycle.json")).expect("fixture");
    let mut body = String::new();
    for (index, frame) in frames.iter().enumerate() {
        body.push_str(&format!("data: {frame}{}", LF.repeat(2)));
        if index == 0 {
            body.push_str(&format!(": heartbeat{}", LF.repeat(2)));
        }
    }
    let Server {
        client, requests, ..
    } = serve(vec![Reply {
        status: "200 OK",
        content_type: "text/event-stream",
        body,
    }]);
    let mut stream = client.events().await.expect("open");
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("event"));
    }
    assert_eq!(events.len(), frames.len());
    assert!(matches!(events[0].data, EventData::ServerConnected));
    assert!(matches!(
        events.last().expect("last").data,
        EventData::SessionDeleted(_)
    ));
    let request = requests.recv().expect("request");
    assert!(request.starts_with("GET /api/event HTTP/1.1"));
    assert!(request.contains("accept: text/event-stream"));
    assert!(
        !request
            .to_ascii_lowercase()
            .contains("x-opencode-directory"),
        "the event stream must be global: {request}"
    );
}

#[tokio::test]
async fn bad_frame_is_an_error_item_and_the_stream_goes_on() {
    let body = [
        r#"data: {"id":"e1","type":"server.connected","data":{}}"#,
        "data: not json",
        r#"data: {"id":"e2","created":1,"type":"x.unknown","data":{}}"#,
    ]
    .map(|frame| format!("{frame}{}", LF.repeat(2)))
    .concat();
    let server = serve(vec![Reply {
        status: "200 OK",
        content_type: "text/event-stream",
        body,
    }]);
    let client = &server.client;
    let items: Vec<_> = client.events().await.expect("open").collect().await;
    assert_eq!(items.len(), 3);
    assert!(items[0].is_ok());
    assert!(matches!(items[1], Err(ClientError::Decode(_))));
    assert!(matches!(&items[2], Ok(e) if matches!(e.data, EventData::Raw { .. })));
}

#[tokio::test]
async fn event_stream_open_failure_is_a_status_error() {
    let server = serve(vec![Reply {
        status: "503 Service Unavailable",
        content_type: "text/plain",
        body: "starting".into(),
    }]);
    let client = &server.client;
    assert!(matches!(
        client.events().await,
        Err(ClientError::Status { status: 503, .. })
    ));
}

/// An event stream that writes `chunks` with `gap` between them, then holds the connection
/// open without a byte for up to five seconds. The handle is only held, never joined: the
/// client's connection task runs on the test's one runtime thread.
fn trickle(chunks: Vec<String>, gap: Duration, timeout: Duration) -> (Client, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        read_request(&mut stream);
        let head = ["HTTP/1.1 200 OK", "content-type: text/event-stream", "", ""].join(CRLF);
        stream.write_all(head.as_bytes()).expect("write head");
        for chunk in chunks {
            stream.write_all(chunk.as_bytes()).expect("write chunk");
            stream.flush().expect("flush");
            thread::sleep(gap);
        }
        // Silent but open: the client must give up by itself.
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).ok();
    });
    let client =
        Client::with_base_url(format!("http://127.0.0.1:{port}"), timeout).expect("client");
    (client, worker)
}

#[tokio::test]
async fn a_silent_event_stream_ends_with_an_idle_error() {
    let connected = format!(
        r#"data: {{"id":"e1","type":"server.connected","data":{{}}}}{}"#,
        LF.repeat(2)
    );
    let (client, _worker) = trickle(vec![connected], Duration::ZERO, Duration::from_secs(5));
    let client = client.with_event_idle_timeout(Duration::from_millis(300));

    let items: Vec<_> = client.events().await.expect("open").collect().await;

    assert_eq!(items.len(), 2, "{items:?}");
    assert!(items[0].is_ok());
    assert!(matches!(items[1], Err(ClientError::EventsIdle { .. })));
}

#[tokio::test]
async fn heartbeats_keep_the_stream_past_the_call_timeout() {
    // Heartbeats every 100 ms outlast a 300 ms call timeout only because the body is not
    // bounded by it; with the old request-level timeout this failed at 300 ms.
    let heartbeats = vec![format!(": heartbeat{}", LF.repeat(2)); 5];
    let (client, _worker) = trickle(
        heartbeats,
        Duration::from_millis(100),
        Duration::from_millis(300),
    );
    let client = client.with_event_idle_timeout(Duration::from_millis(400));
    let mut stream = client.events().await.expect("open");
    // Five heartbeats 100 ms apart outlast the 400 ms limit only because each one counts.
    let first = tokio::time::timeout(Duration::from_millis(700), stream.next()).await;

    assert!(first.is_err(), "no item while heartbeats arrive: {first:?}");
    let ended = stream.next().await;
    assert!(matches!(ended, Some(Err(ClientError::EventsIdle { .. }))));
    assert!(
        stream.next().await.is_none(),
        "the stream ends after the idle error"
    );
}
