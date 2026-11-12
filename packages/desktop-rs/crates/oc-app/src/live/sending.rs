//! Runs one composer send's plan in the I/O task (design "Send decision", "Dismiss on send").
//!
//! The dismissals run in parallel before anything else; a 404 means the request is already
//! settled and anything else restores it from the record the window sent along. A failed
//! dismissal never blocks the send. Then the plan either queues the text or takes the whole
//! queue and sends the joined texts.

use futures::channel::mpsc::Sender;
use oc_client::permission::PermissionReply;
use oc_client::queue::{EnqueueItem, QueueUpdate, SendConfig};
use oc_client::types::Delivery;
use oc_client::{Client, ClientError, Directory};
use oc_state::{Action, DismissKind, join_queue_text, taken_texts};

use super::execution::{DismissCall, SendAction, SendExecution};
use super::protocol::{Closed, Update, apply, emit};
use super::send::{OpenSend, prompt};

/// Runs `send` in plan order.
pub(crate) async fn execute(
    client: &Client,
    send: SendExecution,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    match &send.action {
        SendAction::Enqueue { text } => enqueue(client, &send, text, updates).await,
        SendAction::DismissThenEnqueue { text } => {
            dismiss(client, &send.dismiss, updates).await?;
            match text {
                Some(text) => enqueue(client, &send, text, updates).await,
                None => emit(updates, Update::SendStopped).await,
            }
        }
        SendAction::DismissThenSend {
            take_queue,
            text,
            steer,
        } => {
            dismiss(client, &send.dismiss, updates).await?;
            let text = if *take_queue {
                match client.take_all(send.session.as_str()).await {
                    Ok(taken) => {
                        let joined = join_queue_text(taken_texts(&taken.items), text);
                        let update = QueueUpdate {
                            revision: taken.revision,
                            session: taken.session,
                        };
                        emit(updates, Update::QueueUpdated(Box::new(update))).await?;
                        joined
                    }
                    Err(error) => {
                        let message = format!("Could not take the queued messages: {error}");
                        return apply(updates, Action::PromptFailed(message)).await;
                    }
                }
            } else {
                text.clone()
            };
            send_now(client, &send, &text, *steer, updates).await
        }
    }
}

/// Answers every pending request; a 404 is already settled and anything else goes back to the
/// window, which restores the request. A failure never blocks the send.
async fn dismiss(
    client: &Client,
    calls: &[DismissCall],
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let answers =
        futures::future::join_all(calls.iter().map(|call| dismiss_one(client, call))).await;
    for (call, answer) in calls.iter().zip(answers) {
        if let Err(message) = answer {
            let update = Update::DismissFailed {
                call: Box::new(call.clone()),
                message,
            };
            emit(updates, update).await?;
        }
    }
    Ok(())
}

async fn dismiss_one(client: &Client, call: &DismissCall) -> Result<(), String> {
    let directory = Directory::new(call.directory.clone());
    let answer = match call.kind {
        DismissKind::Permission => {
            client
                .reply_permission(
                    &directory,
                    &call.session_id,
                    &call.id,
                    PermissionReply::Reject,
                )
                .await
        }
        DismissKind::Form => {
            client
                .cancel_form(&directory, &call.session_id, &call.id)
                .await
        }
    };
    match answer {
        Ok(()) => Ok(()),
        Err(ClientError::NotFound { .. }) => Ok(()),
        Err(error) => Err(format!("Could not dismiss the pending request: {error}")),
    }
}

/// Queues the text with the composer's model, agent and variant.
async fn enqueue(
    client: &Client,
    send: &SendExecution,
    text: &str,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let directory = Directory::new(send.directory.clone());
    let queued_item = EnqueueItem {
        content: text.to_owned(),
        text: text.to_owned(),
        send_config: SendConfig {
            provider_id: send
                .model
                .as_ref()
                .map(|model| model.provider_id.clone())
                .unwrap_or_default(),
            model_id: send
                .model
                .as_ref()
                .map(|model| model.id.clone())
                .unwrap_or_default(),
            agent: send.agent.clone(),
            variant: send.model.as_ref().and_then(|model| model.variant.clone()),
        },
    };
    match client
        .enqueue(&directory, send.session.as_str(), &queued_item)
        .await
    {
        Ok(result) => {
            let update = oc_client::queue::QueueUpdate {
                revision: result.revision,
                session: result.session,
            };
            emit(updates, Update::QueueUpdated(Box::new(update))).await?;
            emit(
                updates,
                Update::Queued {
                    text: text.to_owned(),
                },
            )
            .await
        }
        Err(error) => {
            let message = format!("Could not queue the message: {error}");
            apply(updates, Action::PromptFailed(message)).await
        }
    }
}

/// Switches the picks, then sends the text as a steer or a plain prompt.
async fn send_now(
    client: &Client,
    send: &SendExecution,
    text: &str,
    steer: bool,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let open = OpenSend {
        session: &send.session,
        directory: &send.directory,
        text,
        agent: send.agent.as_deref(),
        model: send.model.as_ref(),
        record_model: send.record_model.as_ref(),
        delivery: steer.then_some(Delivery::Steer),
    };
    prompt(client, open, updates).await
}

#[cfg(test)]
#[path = "sending_test.rs"]
mod sending_test;
