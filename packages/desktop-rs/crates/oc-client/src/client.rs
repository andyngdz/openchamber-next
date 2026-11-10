//! The calls the desktop spike makes against the OpenChamber server.

use std::pin::Pin;
use std::time::Duration;

use eventsource_stream::{EventStreamError, Eventsource};
use futures::{Stream, StreamExt};
use reqwest::header::ACCEPT;
use reqwest::{RequestBuilder, Response};
use serde::de::DeserializeOwned;

use crate::error::{ByteError, ClientError};
use crate::event::Event;
use crate::message_id::new_message_id;
use crate::types::{
    AuthStatus, CreateSessionRequest, Data, Location, Message, Page, PromptAccepted, PromptRequest,
    Session,
};
use crate::uri_component::encode_uri_component;

const DIRECTORY_HEADER: &str = "x-opencode-directory";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// The server writes `:heartbeat` on the event stream every 20 s
/// (`DEFAULT_SSE_HEARTBEAT_INTERVAL_MS` in `packages/web/server/lib/opencode/proxy.js`). Three
/// missed heartbeats in a row mean the connection is dead even when the socket never says so.
const EVENT_IDLE_TIMEOUT: Duration = Duration::from_secs(3 * 20);
/// Same page size the React UI asks for.
const SESSION_LIMIT: u32 = 50;

/// Decoded events of `GET /api/event`. Dropping the stream closes the connection.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<Event, ClientError>> + Send>>;

/// One server and one directory. Every OpenCode call carries the directory header.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    directory: String,
    timeout: Duration,
    event_idle: Duration,
}

impl Client {
    /// A client for the sidecar on `http://127.0.0.1:<port>`.
    pub fn new(port: u16, directory: impl Into<String>) -> Result<Self, ClientError> {
        Self::with_base_url(
            format!("http://127.0.0.1:{port}"),
            directory,
            REQUEST_TIMEOUT,
        )
    }

    /// A client for any base URL and call timeout. The cookie store is on so a later
    /// stage can add password login without changing the client.
    pub fn with_base_url(
        base_url: impl Into<String>,
        directory: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, ClientError> {
        // The server is always on this machine; a proxy from the environment must not
        // intercept it.
        let http = reqwest::Client::builder()
            .no_proxy()
            .cookie_store(true)
            .connect_timeout(timeout)
            .build()
            .map_err(ClientError::Transport)?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            directory: directory.into(),
            timeout,
            event_idle: EVENT_IDLE_TIMEOUT,
        })
    }

    /// The same client with another event-stream idle limit, see [`Client::events`].
    pub fn with_event_idle_timeout(mut self, idle: Duration) -> Self {
        self.event_idle = idle;
        self
    }

    /// `GET /auth/session`; anything but `authenticated: true` is [`ClientError::Auth`].
    pub async fn check_auth(&self) -> Result<(), ClientError> {
        let status: AuthStatus = self.get("/auth/session").await?;
        if status.authenticated {
            Ok(())
        } else {
            Err(ClientError::Auth)
        }
    }

    /// `GET /api/session` for the client's directory.
    pub async fn list_sessions(&self) -> Result<Vec<Session>, ClientError> {
        let path = format!(
            "/api/session?limit={SESSION_LIMIT}&directory={}",
            encode_uri_component(&self.directory)
        );
        let page: Page<Session> = self.get(&path).await?;
        Ok(page.data)
    }

    /// `POST /api/session` in the client's directory.
    pub async fn create_session(&self) -> Result<Session, ClientError> {
        let body = CreateSessionRequest {
            location: Location {
                directory: self.directory.clone(),
            },
        };
        let created: Data<Session> = self.send(self.post("/api/session").json(&body)).await?;
        Ok(created.data)
    }

    /// `GET /api/session/:id/message`, newest first.
    pub async fn list_messages(&self, session_id: &str) -> Result<Vec<Message>, ClientError> {
        let page: Page<Message> = self
            .get(&format!(
                "/api/session/{}/message",
                encode_uri_component(session_id)
            ))
            .await?;
        Ok(page.data)
    }

    /// `POST /api/session/:id/prompt` with a freshly minted message id.
    pub async fn send_prompt(
        &self,
        session_id: &str,
        text: &str,
    ) -> Result<PromptAccepted, ClientError> {
        let body = PromptRequest {
            id: new_message_id(),
            text: text.to_owned(),
        };
        let path = format!("/api/session/{}/prompt", encode_uri_component(session_id));
        let accepted: Data<PromptAccepted> = self.send(self.post(&path).json(&body)).await?;
        Ok(accepted.data)
    }

    /// `GET /api/event`. The connect and the response head have the call timeout. After that
    /// the stream ends when the server closes it, when the caller drops it, or with
    /// [`ClientError::EventsIdle`] when nothing (no event, no heartbeat comment) arrives for
    /// the idle limit, 60 s unless [`Client::with_event_idle_timeout`] set another. A frame
    /// that fails to decode is an `Err` item and the stream continues.
    pub async fn events(&self) -> Result<EventStream, ClientError> {
        let request = self
            .http
            .get(self.url("/api/event"))
            .header(DIRECTORY_HEADER, encode_uri_component(&self.directory))
            .header(ACCEPT, "text/event-stream");
        let response = tokio::time::timeout(self.timeout, request.send())
            .await
            .map_err(|_| ClientError::Timeout {
                seconds: self.timeout.as_secs(),
            })?
            .map_err(ClientError::Transport)?;
        let response = ensure_success(response).await?;
        let idle = self.event_idle;
        let frames = until_idle(Box::pin(response.bytes_stream()), idle)
            .eventsource()
            .map(move |frame| match frame {
                Ok(frame) => {
                    serde_json::from_str::<Event>(&frame.data).map_err(ClientError::Decode)
                }
                Err(EventStreamError::Transport(ByteError::Transport(err))) => {
                    Err(ClientError::Transport(err))
                }
                Err(EventStreamError::Transport(ByteError::Idle)) => Err(ClientError::EventsIdle {
                    seconds: idle.as_secs(),
                }),
                Err(other) => Err(ClientError::Stream(other.to_string())),
            });
        Ok(Box::pin(frames))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn request(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .header(DIRECTORY_HEADER, encode_uri_component(&self.directory))
            .timeout(self.timeout)
    }

    fn post(&self, path: &str) -> RequestBuilder {
        self.request(self.http.post(self.url(path)))
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        self.send(self.request(self.http.get(self.url(path)))).await
    }

    async fn send<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T, ClientError> {
        let response = request.send().await.map_err(ClientError::Transport)?;
        let response = ensure_success(response).await?;
        let body = response.bytes().await.map_err(ClientError::Transport)?;
        serde_json::from_slice(&body).map_err(ClientError::Decode)
    }
}

/// The body's chunks, ending with [`ByteError::Idle`] when none arrives within `idle`. Bytes
/// are counted before SSE parsing, so a heartbeat comment, which the parser drops, still
/// counts as life.
fn until_idle<S, B>(body: S, idle: Duration) -> impl Stream<Item = Result<B, ByteError>>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
{
    futures::stream::unfold(Some(body), move |body| async move {
        let mut body = body?;
        match tokio::time::timeout(idle, body.next()).await {
            Ok(Some(chunk)) => Some((chunk.map_err(ByteError::Transport), Some(body))),
            Ok(None) => None,
            Err(_) => Some((Err(ByteError::Idle), None)),
        }
    })
}

async fn ensure_success(response: Response) -> Result<Response, ClientError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(ClientError::Status {
        status: status.as_u16(),
        body,
    })
}

#[cfg(test)]
#[path = "client_test.rs"]
mod client_test;
