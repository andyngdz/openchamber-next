//! The SSE reader's idle behavior, on a server that trickles frames and then holds the
//! connection open.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use futures::StreamExt;

use crate::client::client_test::read_request;
use crate::{Client, ClientError};

const CRLF: &str = "\r\n";
const LF: &str = "\n";

/// An event stream that writes `chunks` with `gap` between them, then holds the connection
/// open without a byte for up to five seconds. The handle is only held, never joined: the
/// client's connection task runs on the test's one runtime thread.
pub(crate) fn trickle(
    chunks: Vec<String>,
    gap: Duration,
    timeout: Duration,
) -> (Client, JoinHandle<()>) {
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
