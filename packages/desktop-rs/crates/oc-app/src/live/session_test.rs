//! The session task against a one-shot HTTP server on a free port.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::{Client, Directory};
use oc_state::{Action, ConnectionStatus, SessionId};

use super::{answer, run_session};
use crate::live::execution::{SendAction, SendExecution};
use crate::live::protocol::{Command, CreateSessionRequest, CreateTarget, Update};

const TIMEOUT: Duration = Duration::from_secs(5);
const DIRECTORY: &str = "/tmp/p";

/// Answers each connection with the next `(status line, JSON body)`, then stops.
fn serve(replies: Vec<(&'static str, &'static str)>) -> (Client, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let worker = thread::spawn(move || {
        for (status, body) in replies {
            let (stream, _) = listener.accept().expect("accept");
            reply(stream, status, body);
        }
    });
    let client =
        Client::with_base_url(format!("http://127.0.0.1:{port}"), TIMEOUT).expect("client");
    (client, worker)
}

fn reply(mut stream: TcpStream, status: &str, body: &str) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("read head");
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().expect("length");
        }
    }
    let mut request_body = vec![0; length];
    reader.read_exact(&mut request_body).expect("read body");
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write");
}

async fn answers_of(replies: Vec<(&'static str, &'static str)>, command: Command) -> Vec<Update> {
    let (client, worker) = serve(replies);
    let (sender, receiver) = mpsc::channel(4);
    answer(client, command, sender).await;
    worker.join().expect("server thread");
    receiver.collect().await
}

async fn answer_of(status: &'static str, body: &'static str, command: Command) -> Action {
    let updates = answers_of(vec![(status, body)], command).await;
    match updates.as_slice() {
        [Update::Apply(action)] => action.clone(),
        other => panic!("one action expected, got {other:?}"),
    }
}

fn project_target() -> CreateTarget {
    CreateTarget::Project {
        directory: DIRECTORY.to_owned(),
    }
}

fn prompt(text: &str) -> Command {
    Command::Send(Box::new(SendExecution {
        session: SessionId::from("ses_a"),
        directory: DIRECTORY.to_owned(),
        agent: None,
        model: None,
        record_model: None,
        dismiss: Vec::new(),
        action: SendAction::DismissThenSend {
            take_queue: false,
            text: text.to_owned(),
            steer: false,
        },
    }))
}

#[tokio::test]
async fn an_unauthenticated_server_fails_the_connection_and_ends_the_task() {
    let (client, worker) = serve(vec![("200 OK", r#"{"authenticated":false}"#)]);
    let (_commands, command_receiver) = mpsc::channel(1);
    let (sender, receiver) = mpsc::channel(4);

    run_session(client, command_receiver, sender).await;
    worker.join().expect("server thread");

    let updates: Vec<Update> = receiver.collect().await;
    let [Update::Apply(Action::Connection(ConnectionStatus::Failed(failure)))] = updates.as_slice()
    else {
        panic!("a failed connection expected, got {updates:?}");
    };
    assert!(
        failure.message.starts_with("Server check failed"),
        "{}",
        failure.message
    );
}

#[tokio::test]
async fn a_failed_message_list_is_a_region_error_never_an_empty_list() {
    let command = Command::LoadMessages {
        session: SessionId::from("ses_a"),
        directory: DIRECTORY.to_owned(),
    };
    let action = answer_of("500 Internal Server Error", r#"{"error":"boom"}"#, command).await;

    let Action::MessagesFailed { message, .. } = action else {
        panic!("MessagesFailed expected, got {action:?}");
    };
    assert!(message.contains("500"), "{message}");
}

#[tokio::test]
async fn a_failed_create_is_its_own_error_not_a_list_error() {
    let command = Command::CreateSession(Box::new(CreateSessionRequest {
        target: project_target(),
        agent: None,
        model: None,
    }));
    let action = answer_of("500 Internal Server Error", "{}", command).await;

    assert!(
        matches!(&action, Action::CreateFailed(message) if message.contains("500")),
        "{action:?}"
    );
}

#[tokio::test]
async fn an_accepted_prompt_names_the_text_it_sent() {
    let updates = answers_of(
        vec![("200 OK", r#"{"data":{"id":"msg_1"}}"#)],
        prompt("hello"),
    )
    .await;

    assert_eq!(
        updates,
        [Update::Sent {
            text: "hello".to_owned(),
        }]
    );
}

#[tokio::test]
async fn a_failed_prompt_reports_prompt_failed() {
    let updates = answers_of(vec![("503 Service Unavailable", "{}")], prompt("hello")).await;

    assert!(
        matches!(updates.as_slice(), [Update::Apply(Action::PromptFailed(_))]),
        "{updates:?}"
    );
}

#[tokio::test]
async fn loaded_messages_name_their_session() {
    let command = Command::LoadMessages {
        session: SessionId::from("ses_a"),
        directory: DIRECTORY.to_owned(),
    };
    let action = answer_of("200 OK", r#"{"data":[]}"#, command).await;

    assert_eq!(
        action,
        Action::MessagesLoaded {
            session: SessionId::from("ses_a"),
            messages: Vec::new(),
        }
    );
}

#[tokio::test]
async fn a_missing_session_record_is_logged_not_failed() {
    let command = Command::LoadSessionRecord {
        session: SessionId::from("ses_a"),
        directory: Directory::new(DIRECTORY).as_str().to_owned(),
    };
    let updates = answers_of(vec![("404 Not Found", "{}")], command).await;

    assert!(updates.is_empty(), "{updates:?}");
}
