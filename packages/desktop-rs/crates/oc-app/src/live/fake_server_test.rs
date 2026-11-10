//! A short-lived HTTP server for the live request tests: one scripted reply per connection,
//! and every request recorded so a test can assert what was sent.

#![cfg(test)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use oc_client::Client;

const TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(2);

/// A switch a test flips to let one scripted reply through, so a response lands after a
/// later request was sent.
#[derive(Clone)]
pub(crate) struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    /// A gate that holds its reply until `open`.
    pub(crate) fn closed() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }

    /// A gate that lets its reply through at once.
    pub(crate) fn passing() -> Self {
        let gate = Self::closed();
        gate.open();
        gate
    }

    /// Lets the held reply through; later waits return at once.
    pub(crate) fn open(&self) {
        let (open, ready) = &*self.0;
        *open.lock().expect("gate lock") = true;
        ready.notify_all();
    }

    fn wait(&self) {
        let (open, ready) = &*self.0;
        let mut open = open.lock().expect("gate lock");
        while !*open {
            open = ready.wait(open).expect("gate wait");
        }
    }
}

/// One request the server answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) body: String,
}

impl Request {
    /// The body decoded as JSON.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap_or(serde_json::Value::Null)
    }
}

/// A server scripted with `(status line, JSON body)` replies, in connection order.
pub(crate) struct FakeServer {
    pub(crate) client: Client,
    pub(crate) worker: JoinHandle<()>,
    requests: Arc<Mutex<Vec<Request>>>,
    accepted: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl FakeServer {
    pub(crate) fn start(replies: Vec<(&'static str, &'static str)>) -> Self {
        Self::spawn(
            replies
                .into_iter()
                .map(|(status, body)| (status, body, None))
                .collect(),
        )
    }

    /// Like [`Self::start`], but each reply waits for its gate before it answers.
    pub(crate) fn start_gated(replies: Vec<(&'static str, &'static str, Gate)>) -> Self {
        Self::spawn(
            replies
                .into_iter()
                .map(|(status, body, gate)| (status, body, Some(gate)))
                .collect(),
        )
    }

    fn spawn(replies: Vec<(&'static str, &'static str, Option<Gate>)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let accepted = Arc::new(AtomicUsize::new(0));
        let arrived = accepted.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let running = stop.clone();
        let worker = thread::spawn(move || {
            for (status, body, gate) in replies {
                let Some(stream) = accept(&listener, &running) else {
                    return;
                };
                arrived.fetch_add(1, Ordering::SeqCst);
                if let Some(gate) = gate {
                    gate.wait();
                }
                let request = answer(stream, status, body);
                recorded.lock().expect("requests").push(request);
            }
        });
        let client =
            Client::with_base_url(format!("http://127.0.0.1:{port}"), TIMEOUT).expect("client");
        Self {
            client,
            worker,
            requests,
            accepted,
            stop,
        }
    }

    /// How many connections the server has accepted so far.
    pub(crate) fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    /// Stops waiting for connections the test will not make, such as the requests
    /// `try_join!` cancels after one fails, then joins the worker and returns every recorded
    /// request.
    pub(crate) fn finish(self) -> Vec<Request> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.join().expect("server thread");
        self.requests.lock().expect("requests").clone()
    }
}

fn accept(listener: &TcpListener, stop: &AtomicBool) -> Option<TcpStream> {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).ok();
                return Some(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(POLL),
            Err(_) => return None,
        }
    }
    None
}

fn answer(mut stream: TcpStream, status: &str, body: &str) -> Request {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut method = String::new();
    let mut path = String::new();
    let mut length = 0;
    let mut first = true;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("read head");
        if first {
            let mut parts = line.split_whitespace();
            method = parts.next().unwrap_or_default().to_owned();
            path = parts.next().unwrap_or_default().to_owned();
            first = false;
        }
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
    Request {
        method,
        path,
        body: String::from_utf8_lossy(&request_body).into_owned(),
    }
}
