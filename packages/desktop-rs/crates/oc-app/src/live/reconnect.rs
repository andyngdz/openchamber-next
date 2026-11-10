//! Follows `GET /api/event` and reconnects it when it drops: 500 ms, doubling up to 10 s.

use std::future::Future;
use std::time::Duration;

use futures::channel::mpsc::Sender;
use futures::{SinkExt, Stream, StreamExt};
use oc_client::{ClientError, Event};
use oc_state::{Action, ConnectionStatus};

use super::protocol::Update;

/// Wait before the first reconnect.
pub(crate) const FIRST_DELAY: Duration = Duration::from_millis(500);
/// Longest wait between reconnects.
pub(crate) const MAX_DELAY: Duration = Duration::from_secs(10);

/// The window stopped listening.
struct Closed;

/// Whether an open stream follows a failed connect or a dropped stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opening {
    First,
    AfterDrop,
}

/// Connects with `connect`, forwards every event, and reconnects after `sleep` whenever the
/// stream fails to open, errors or ends. Each open stream reports Connected; one that follows
/// a failure or a drop also reports [`Update::Reconnected`]. Returns once `updates` is closed.
pub(crate) async fn follow_events<C, CF, St, S, SF>(
    mut connect: C,
    mut sleep: S,
    mut updates: Sender<Update>,
) where
    C: FnMut() -> CF,
    CF: Future<Output = Result<St, ClientError>>,
    St: Stream<Item = Result<Event, ClientError>> + Unpin,
    S: FnMut(Duration) -> SF,
    SF: Future<Output = ()>,
{
    let mut delay = FIRST_DELAY;
    let mut attempt = 0;
    loop {
        match connect().await {
            Ok(stream) => {
                let opening = if attempt > 0 {
                    Opening::AfterDrop
                } else {
                    Opening::First
                };
                attempt = 0;
                delay = FIRST_DELAY;
                if forward(stream, opening, &mut updates).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Event stream did not open");
            }
        }
        attempt += 1;
        let status = ConnectionStatus::Reconnecting { attempt };
        if emit(&mut updates, Update::Apply(Action::Connection(status)))
            .await
            .is_err()
        {
            return;
        }
        sleep(delay).await;
        delay = (delay * 2).min(MAX_DELAY);
    }
}

/// Reads one open stream until it errors or ends. A frame that does not decode is skipped.
async fn forward<St>(
    mut stream: St,
    opening: Opening,
    updates: &mut Sender<Update>,
) -> Result<(), Closed>
where
    St: Stream<Item = Result<Event, ClientError>> + Unpin,
{
    let connected = Update::Apply(Action::Connection(ConnectionStatus::Connected));
    emit(updates, connected).await?;
    match opening {
        Opening::AfterDrop => emit(updates, Update::Reconnected).await?,
        Opening::First => {}
    }
    while let Some(frame) = stream.next().await {
        match frame {
            Ok(event) => emit(updates, Update::Apply(Action::Event(event))).await?,
            Err(ClientError::Decode(error)) => {
                tracing::debug!(target: crate::logging::SERVER, %error, "[SERVER] Skipped an event that does not decode");
            }
            Err(error) => {
                tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Event stream dropped");
                return Ok(());
            }
        }
    }
    tracing::warn!(target: crate::logging::SERVER, "[SERVER] Event stream ended");
    Ok(())
}

async fn emit(updates: &mut Sender<Update>, update: Update) -> Result<(), Closed> {
    updates.send(update).await.map_err(|_| Closed)
}

#[cfg(test)]
#[path = "reconnect_test.rs"]
mod reconnect_test;
