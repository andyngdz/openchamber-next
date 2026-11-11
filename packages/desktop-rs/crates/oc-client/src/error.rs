//! Failures of a client call.

use thiserror::Error;

/// Why a call to the server did not produce a usable answer.
#[derive(Debug, Error)]
pub enum ClientError {
    /// The request never completed: connection refused, reset, timeout.
    #[error("request to the server failed: {0}")]
    Transport(#[source] reqwest::Error),
    /// The server answered with a status outside 2xx.
    #[error("server answered {status}: {body}")]
    Status { status: u16, body: String },
    /// The server answered 404: the route or the record does not exist. A reply or a
    /// cancel treats it as already settled, and a missing routing build as unavailable.
    #[error("server answered 404: {body}")]
    NotFound { body: String },
    /// The body arrived but is not the shape the client expects.
    #[error("could not decode the server answer: {0}")]
    Decode(#[source] serde_json::Error),
    /// The server did not accept the session as authenticated.
    #[error("server does not allow this client: authenticated is false")]
    Auth,
    /// The server accepted the connection but did not answer in time.
    #[error("server did not answer within {seconds} seconds")]
    Timeout { seconds: u64 },
    /// The event stream sent nothing, not even a heartbeat, for this long.
    #[error("event stream sent nothing for {seconds} seconds")]
    EventsIdle { seconds: u64 },
    /// The event stream broke after it had opened.
    #[error("event stream failed: {0}")]
    Stream(String),
}

/// Why the byte stream under the event parser stopped; [`crate::Client::events`] turns it
/// into a [`ClientError`].
#[derive(Debug, Error)]
pub(crate) enum ByteError {
    /// The connection broke.
    #[error(transparent)]
    Transport(reqwest::Error),
    /// Nothing arrived within the idle limit.
    #[error("no bytes within the idle limit")]
    Idle,
}
