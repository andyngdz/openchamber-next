//! The ordered steps a `Prompt` or `Steer` runs, as a value the app executes (design
//! "Send decision").
//!
//! A composer send dismisses the tree's pending requests first (`ChatInput.tsx:2296-2317`),
//! stops after that dismissal when the decision is not a steer (`ChatInput.tsx:1794`), and
//! otherwise takes the whole queue and sends its texts before the composer's own, joined by a
//! blank line (`ChatInput.tsx:2445-2460`, `composer/submit/buildOutgoingMessage.ts:155-180`).
//! `Enqueue` dismisses nothing (`ChatInput.tsx:3199-3205`). The plan claims no I/O: the app
//! runs the steps, and a failed take sends nothing.

use oc_client::queue::QueueItem;
use oc_client::types::Delivery;

use crate::send::{SendDecision, SendKey, decide_send};
use crate::state::State;

/// Which route dismisses one pending request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismissKind {
    /// `POST /api/session/:id/permission/:id/reply` with `reject`.
    Permission,
    /// `DELETE /api/session/:id/form/:id`.
    Form,
}

/// One pending request the send dismisses before its prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DismissTarget {
    /// The session the request belongs to.
    pub session_id: String,
    /// The request or form id.
    pub id: String,
    /// Which call answers it.
    pub kind: DismissKind,
}

/// The prompt that follows the dismissal step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendPrompt {
    /// The session's queue holds items, so take the whole queue first.
    pub take_queue: bool,
    /// The composer text that follows the taken queue in the join.
    pub composer_text: String,
    /// The decision was `Steer`; a runtime idle check downgrades it to a plain prompt.
    pub steer: bool,
}

impl SendPrompt {
    /// `Steer` only while the turn still runs; a plain prompt otherwise
    /// (`ChatInput.tsx:2111`).
    pub fn delivery(&self, idle: bool) -> Option<Delivery> {
        (self.steer && !idle).then_some(Delivery::Steer)
    }
}

/// What the app does for one composer send, in the order it does it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendPlan {
    /// Nothing to send, or the draft's first send is still in flight.
    Nothing,
    /// `Enqueue`: queue the text; dismiss nothing.
    Enqueue {
        /// The text to queue.
        text: String,
    },
    /// Step 2: dismiss these, enqueue `text` when the composer holds any, and stop. The
    /// queue stays for the server to send, and an empty composer enqueues nothing.
    DismissThenEnqueue {
        /// The requests to answer first.
        dismiss: Vec<DismissTarget>,
        /// The composer text to enqueue, when it is not empty.
        text: Option<String>,
    },
    /// Step 3: dismiss these, take the whole queue, and send the joined texts.
    DismissThenSend {
        /// The requests to answer first.
        dismiss: Vec<DismissTarget>,
        /// The prompt after the queue take.
        prompt: SendPrompt,
    },
}

/// A queue row's send-now plan: take that item only and send it, dismissing nothing
/// (`ChatInput.tsx:2065-2068, 2301`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendNowPlan {
    /// The queued item to take.
    pub item_id: String,
    /// `Steer` into the running turn, or a plain prompt when the session is idle.
    pub delivery: Option<Delivery>,
}

/// The send-now plan for one queued row.
pub fn plan_send_now(item_id: &str, idle: bool) -> SendNowPlan {
    SendNowPlan {
        item_id: item_id.to_owned(),
        delivery: (!idle).then_some(Delivery::Steer),
    }
}

/// The joined text of a send: the queued texts in queue order, then the composer text,
/// separated by a blank line (`buildOutgoingMessage.ts:155-180`).
///
/// A blank queued text is skipped, and the composer text loses its leading and trailing
/// newlines as React does; a blank composer text adds nothing.
pub fn join_queue_text<'a>(
    queued: impl IntoIterator<Item = &'a str>,
    composer_text: &str,
) -> String {
    let mut texts: Vec<&str> = queued
        .into_iter()
        .filter(|text| !text.trim().is_empty())
        .collect();
    let composer = composer_text.trim_matches('\n');
    if !composer.trim().is_empty() {
        texts.push(composer);
    }
    texts.join(&"\n".repeat(2))
}

impl State {
    /// The plan for a composer send with `key`. The follow-up button above Stop passes
    /// [`SendKey::Plain`] even when `enterToSend` is off, so it still applies
    /// `followUpBehavior`.
    pub fn send_plan(&self, key: SendKey) -> SendPlan {
        let input = self.send_input(key);
        let decision = decide_send(&input);
        match decision {
            SendDecision::None => SendPlan::Nothing,
            SendDecision::Enqueue => SendPlan::Enqueue { text: input.text },
            SendDecision::Prompt | SendDecision::Steer => {
                let steer = decision == SendDecision::Steer;
                let dismiss = self.dismiss_targets();
                let text = (!input.text.trim().is_empty()).then_some(input.text);
                if dismiss.is_empty() || steer {
                    SendPlan::DismissThenSend {
                        dismiss,
                        prompt: SendPrompt {
                            take_queue: input.has_queue,
                            composer_text: text.unwrap_or_default(),
                            steer,
                        },
                    }
                } else {
                    SendPlan::DismissThenEnqueue { dismiss, text }
                }
            }
        }
    }

    /// The pending requests of the open session's tree, permissions then forms.
    fn dismiss_targets(&self) -> Vec<DismissTarget> {
        let Some(open) = self.open.as_ref().map(|open| open.id.clone()) else {
            return Vec::new();
        };
        let mut targets = Vec::new();
        for session in self.subtree_ids(open.as_str()) {
            targets.extend(
                self.requests
                    .permissions_for(&session)
                    .iter()
                    .map(|request| DismissTarget {
                        session_id: session.clone(),
                        id: request.id.clone(),
                        kind: DismissKind::Permission,
                    }),
            );
            targets.extend(
                self.requests
                    .forms_for(&session)
                    .iter()
                    .map(|form| DismissTarget {
                        session_id: session.clone(),
                        id: form.id.clone(),
                        kind: DismissKind::Form,
                    }),
            );
        }
        targets
    }
}

/// The texts of the items a take returned, in queue order.
pub fn taken_texts(items: &[QueueItem]) -> Vec<&str> {
    items.iter().map(|item| item.text.as_str()).collect()
}

#[cfg(test)]
#[path = "send_plan_test.rs"]
mod send_plan_test;
