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
    /// The body arrived but is not the shape the client expects.
    #[error("could not decode the server answer: {0}")]
    Decode(#[source] serde_json::Error),
    /// The server did not accept the session as authenticated.
    #[error("server does not allow this client: authenticated is false")]
    Auth,
    /// The server accepted the connection but did not answer in time.
    #[error("server did not answer within {seconds} seconds")]
    Timeout { seconds: u64 },
    /// The event stream broke after it had opened.
    #[error("event stream failed: {0}")]
    Stream(String),
}
