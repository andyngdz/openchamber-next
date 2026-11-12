//! The byte stream under an SSE connection, shared by both server streams.

use std::time::Duration;

use futures::{Stream, StreamExt};

use crate::error::ByteError;

/// The body's chunks, ending with [`ByteError::Idle`] when none arrives within `idle`.
/// Bytes are counted before SSE parsing, so a heartbeat comment, which the parser drops,
/// still counts as life.
pub(crate) fn until_idle<S, B>(body: S, idle: Duration) -> impl Stream<Item = Result<B, ByteError>>
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

#[cfg(test)]
#[path = "sse_test.rs"]
pub(crate) mod sse_test;
