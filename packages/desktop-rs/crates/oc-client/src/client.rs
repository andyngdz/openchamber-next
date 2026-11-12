//! The calls the desktop app makes against the OpenChamber server.
//!
//! One client talks to one server; the directory a scoped call runs in is passed per call,
//! and [`Client::events`] follows the one global event stream.

use std::pin::Pin;
use std::time::Duration;

use eventsource_stream::{EventStreamError, Eventsource};
use futures::{Stream, StreamExt};
use reqwest::header::ACCEPT;
use reqwest::{RequestBuilder, Response};
use serde::de::DeserializeOwned;

use crate::control::ControlEvent;
use crate::directory::Directory;
use crate::error::{ByteError, ClientError};
use crate::event::Event;
use crate::sse::until_idle;
use crate::types::AuthStatus;
use crate::uri_component::encode_uri_component;

pub(crate) const DIRECTORY_HEADER: &str = "x-opencode-directory";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// The server writes `:heartbeat` on the event stream every 20 s
/// (`DEFAULT_SSE_HEARTBEAT_INTERVAL_MS` in `packages/web/server/lib/opencode/proxy.js`). Three
/// missed heartbeats in a row mean the connection is dead even when the socket never says so.
const EVENT_IDLE_TIMEOUT: Duration = Duration::from_secs(3 * 20);
/// The control stream counts 45 s without a frame as a drop
/// (`HEARTBEAT_TIMEOUT_MS` in `packages/ui/src/lib/openchamberEvents.ts`).
const CONTROL_IDLE_TIMEOUT: Duration = Duration::from_secs(45);

/// Decoded events of `GET /api/event`. Dropping the stream closes the connection.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<Event, ClientError>> + Send>>;

/// Decoded events of `GET /api/openchamber/events`. Dropping the stream closes the
/// connection.
pub type ControlStream = Pin<Box<dyn Stream<Item = Result<ControlEvent, ClientError>> + Send>>;

/// One server. Scoped calls carry the directory they run in; the event stream is global.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    timeout: Duration,
    event_idle: Duration,
    control_idle: Duration,
}

impl Client {
    /// A client for the sidecar on `http://127.0.0.1:<port>`.
    pub fn new(port: u16) -> Result<Self, ClientError> {
        Self::with_base_url(format!("http://127.0.0.1:{port}"), REQUEST_TIMEOUT)
    }

    /// A client for any base URL and call timeout. The cookie store is on so a later
    /// stage can add password login without changing the client.
    pub fn with_base_url(
        base_url: impl Into<String>,
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
            timeout,
            event_idle: EVENT_IDLE_TIMEOUT,
            control_idle: CONTROL_IDLE_TIMEOUT,
        })
    }

    /// The same client with another event-stream idle limit, see [`Client::events`].
    pub fn with_event_idle_timeout(mut self, idle: Duration) -> Self {
        self.event_idle = idle;
        self
    }

    /// The same client with another control-stream idle limit, see
    /// [`Client::control_events`].
    pub fn with_control_idle_timeout(mut self, idle: Duration) -> Self {
        self.control_idle = idle;
        self
    }

    /// `GET /auth/session`; anything but `authenticated: true` is [`ClientError::Auth`].
    pub async fn check_auth(&self) -> Result<(), ClientError> {
        let status: AuthStatus = self.get_unscoped("/auth/session").await?;
        if status.authenticated {
            Ok(())
        } else {
            Err(ClientError::Auth)
        }
    }

    /// `GET /api/event` with no directory, so the stream is global: every project's events
    /// arrive here, each naming its directory. The connect and the response head have the
    /// call timeout; the body is not limited by it, only by the idle limit below. The stream
    /// ends when the server closes it, when the caller drops it, or with
    /// [`ClientError::EventsIdle`] when nothing (no event, no heartbeat comment) arrives for
    /// the idle limit, 60 s unless [`Client::with_event_idle_timeout`] set another. A frame
    /// that fails to decode is an `Err` item and the stream continues.
    pub async fn events(&self) -> Result<EventStream, ClientError> {
        self.stream("/api/event", self.event_idle, |frame| {
            serde_json::from_str::<Event>(frame).map_err(ClientError::Decode)
        })
        .await
    }

    /// `GET /api/openchamber/events`: the server's own control stream, with the same auth
    /// and connection rules as [`Client::events`]. Frames are `data:` lines only, each a
    /// flat `{type, properties}`. The stream ends with [`ClientError::EventsIdle`] when no
    /// frame arrives for 45 s, unless [`Client::with_control_idle_timeout`] set another. An
    /// older server without the stream answers 404, which is [`ClientError::NotFound`].
    pub async fn control_events(&self) -> Result<ControlStream, ClientError> {
        self.stream("/api/openchamber/events", self.control_idle, |frame| {
            serde_json::from_str::<ControlEvent>(frame).map_err(ClientError::Decode)
        })
        .await
    }

    /// Opens an SSE stream and decodes every `data:` frame with `decode`. A frame that
    /// fails to decode is an `Err` item and the stream continues.
    async fn stream<T: Send + 'static>(
        &self,
        path: &str,
        idle: Duration,
        decode: fn(&str) -> Result<T, ClientError>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<T, ClientError>> + Send>>, ClientError> {
        let request = self
            .http
            .get(self.url(path))
            .header(ACCEPT, "text/event-stream");
        let response = tokio::time::timeout(self.timeout, request.send())
            .await
            .map_err(|_| ClientError::Timeout {
                seconds: self.timeout.as_secs(),
            })?
            .map_err(ClientError::Transport)?;
        let response = ensure_success(response).await?;
        let frames = until_idle(Box::pin(response.bytes_stream()), idle)
            .eventsource()
            .map(move |frame| match frame {
                Ok(frame) => decode(&frame.data),
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

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// A builder with the call timeout and no directory: auth, events, home, settings,
    /// the global session list and `mkdir` are not scoped.
    pub(crate) fn request(&self, builder: RequestBuilder) -> RequestBuilder {
        builder.timeout(self.timeout)
    }

    /// A builder carrying the directory header and the call timeout.
    pub(crate) fn scoped(&self, builder: RequestBuilder, directory: &Directory) -> RequestBuilder {
        builder
            .header(DIRECTORY_HEADER, encode_uri_component(directory.as_str()))
            .timeout(self.timeout)
    }

    pub(crate) fn post_unscoped(&self, path: &str) -> RequestBuilder {
        self.request(self.http.post(self.url(path)))
    }

    pub(crate) fn put_unscoped(&self, path: &str) -> RequestBuilder {
        self.request(self.http.put(self.url(path)))
    }

    pub(crate) fn post_scoped(&self, path: &str, directory: &Directory) -> RequestBuilder {
        self.scoped(self.http.post(self.url(path)), directory)
    }

    pub(crate) fn delete_unscoped(&self, path: &str) -> RequestBuilder {
        self.request(self.http.delete(self.url(path)))
    }

    pub(crate) fn delete_scoped(&self, path: &str, directory: &Directory) -> RequestBuilder {
        self.scoped(self.http.delete(self.url(path)), directory)
    }

    pub(crate) async fn get_unscoped<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ClientError> {
        self.send(self.request(self.http.get(self.url(path)))).await
    }

    pub(crate) async fn get_scoped<T: DeserializeOwned>(
        &self,
        path: &str,
        directory: &Directory,
    ) -> Result<T, ClientError> {
        self.send(self.scoped(self.http.get(self.url(path)), directory))
            .await
    }

    pub(crate) async fn send<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T, ClientError> {
        let response = request.send().await.map_err(ClientError::Transport)?;
        let response = ensure_success(response).await?;
        let body = response.bytes().await.map_err(ClientError::Transport)?;
        serde_json::from_slice(&body).map_err(ClientError::Decode)
    }

    /// Sends a request whose answer has no body, such as the 204 model and agent switches.
    pub(crate) async fn send_empty(&self, request: RequestBuilder) -> Result<(), ClientError> {
        let response = request.send().await.map_err(ClientError::Transport)?;
        ensure_success(response).await?;
        Ok(())
    }
}

async fn ensure_success(response: Response) -> Result<Response, ClientError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(ClientError::NotFound { body });
    }
    Err(ClientError::Status {
        status: status.as_u16(),
        body,
    })
}

#[cfg(test)]
#[path = "client_test.rs"]
pub(crate) mod client_test;
