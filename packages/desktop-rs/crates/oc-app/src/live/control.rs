//! The server's own control stream, `GET /api/openchamber/events`, and the snapshots its
//! ready frame asks for (design "Control stream", "Live").
//!
//! The stream carries the message queue, the auto-accept policy, the routing state and the
//! requests the server left for the user. A drop reconnects after 1 second, doubling up to
//! 30; a server without the route answers 404, which ends the task for good.

use std::future::Future;
use std::time::Duration;

use futures::channel::mpsc::Sender;
use futures::{SinkExt, Stream, StreamExt};
use oc_client::control::ControlEvent;
use oc_client::{Client, ClientError};

use super::protocol::{Closed, Update};

/// Wait before the first reconnect.
pub(crate) const FIRST_DELAY: Duration = Duration::from_secs(1);
/// Longest wait between reconnects.
pub(crate) const MAX_DELAY: Duration = Duration::from_secs(30);

/// Connects with `connect` and forwards every frame. A dropped or failed stream waits before
/// it reconnects from the first delay; a 404 ends the loop, because the server has no control
/// stream at all. Returns once `updates` is closed or the route is missing.
pub(crate) async fn follow<C, CF, St, S, SF>(
    mut connect: C,
    mut sleep: S,
    mut updates: Sender<Update>,
) where
    C: FnMut() -> CF,
    CF: Future<Output = Result<St, ClientError>>,
    St: Stream<Item = Result<ControlEvent, ClientError>> + Unpin,
    S: FnMut(Duration) -> SF,
    SF: Future<Output = ()>,
{
    let mut delay = FIRST_DELAY;
    loop {
        match connect().await {
            Ok(stream) => {
                delay = FIRST_DELAY;
                if forward(stream, &mut updates).await.is_err() {
                    return;
                }
            }
            Err(ClientError::NotFound { .. }) => {
                tracing::warn!(
                    target: crate::logging::SERVER,
                    "[SERVER] The server has no control stream"
                );
                updates.send(Update::ControlStreamMissing).await.ok();
                return;
            }
            Err(error) => {
                tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Control stream did not open");
            }
        }
        sleep(delay).await;
        delay = (delay * 2).min(MAX_DELAY);
    }
}

/// Reads one open stream until it errors or ends. A frame that does not decode is skipped.
async fn forward<St>(
    mut stream: St,
    updates: &mut Sender<Update>,
) -> Result<(), futures::channel::mpsc::SendError>
where
    St: Stream<Item = Result<ControlEvent, ClientError>> + Unpin,
{
    while let Some(frame) = stream.next().await {
        match frame {
            Ok(event) => updates.send(Update::Control(event)).await?,
            Err(ClientError::Decode(error)) => {
                tracing::debug!(target: crate::logging::SERVER, %error, "[SERVER] Skipped a control frame that does not decode");
            }
            Err(error) => {
                tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Control stream dropped");
                return Ok(());
            }
        }
    }
    tracing::warn!(target: crate::logging::SERVER, "[SERVER] Control stream ended");
    Ok(())
}

/// Re-reads the three snapshots a ready frame asks for. Each answer is applied on its own, so
/// one failure keeps the others and the state's previous values.
pub(crate) async fn refresh(client: &Client, updates: &mut Sender<Update>) -> Result<(), Closed> {
    let queue = client
        .message_queue()
        .await
        .map_err(|error| format!("Could not read the message queue: {error}"));
    emit(updates, Update::QueueLoaded(queue)).await?;
    let auto_accept = client
        .auto_accept()
        .await
        .map_err(|error| format!("Could not read the permission modes: {error}"));
    emit(updates, Update::AutoAcceptLoaded(auto_accept)).await?;
    let routing = client
        .routing()
        .await
        .map_err(|error| format!("Could not read the routing state: {error}"));
    emit(updates, Update::RoutingLoaded(routing)).await
}

async fn emit(updates: &mut Sender<Update>, update: Update) -> Result<(), Closed> {
    updates.send(update).await.map_err(|_| Closed)
}

#[cfg(test)]
#[path = "control_test.rs"]
mod control_test;
