//! The composer's send decision (`ChatInput.tsx:2101-2111, 2883-2896, 3185-3213`; design
//! "Send decision").
//!
//! Pure: the app assembles [`SendInput`] from state and the key it admitted through
//! [`crate::should_submit_enter`], then runs the [`crate::SendPlan`] that [`State::send_plan`]
//! builds. `followUpBehavior` decides only the plain send key; Ctrl/Cmd+Enter always sends
//! a plain prompt, which is why a composer with `enterToSend` off never enqueues or steers
//! from the keyboard.

use oc_client::follow_up::FollowUpBehavior;
use oc_client::settings::Settings;

use crate::state::State;
use crate::status::{ComposerStatus, SessionStatus};

/// Which send key produced the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendKey {
    /// Enter, or whatever `should_submit_enter` admits without a command modifier. The
    /// follow-up button above Stop also uses this key (`ChatInput.tsx:2883-2896`).
    Plain,
    /// Ctrl+Enter or Cmd+Enter.
    Command,
}

/// What the composer does with this send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendDecision {
    /// Send a plain prompt.
    Prompt,
    /// Steer the text into the running turn.
    Steer,
    /// Queue the text in the server's message queue.
    Enqueue,
    /// Nothing: the first send is still in flight, or there is nothing to send.
    None,
}

/// Everything [`decide_send`] reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendInput {
    /// What the composer holds.
    pub text: String,
    /// The open session is not running a turn.
    pub idle: bool,
    /// The settings document's `followUpBehavior`.
    pub follow_up: FollowUpBehavior,
    /// The send key that reached the composer.
    pub key: SendKey,
    /// The draft's first send is in flight.
    pub first_send_pending: bool,
    /// The open session's queue holds items.
    pub has_queue: bool,
}

/// The branch table of design "Send decision" (`ChatInput.tsx:2101-2111, 3199-3213`).
pub fn decide_send(input: &SendInput) -> SendDecision {
    if input.first_send_pending {
        return SendDecision::None;
    }
    let has_text = !input.text.trim().is_empty();
    if !has_text && !input.has_queue {
        // Nothing would go out: `buildOutgoingMessage` is empty and React returns
        // without calling the server (`ChatInput.tsx:2557`).
        return SendDecision::None;
    }
    if !input.idle && has_text && input.key == SendKey::Plain {
        return match input.follow_up {
            FollowUpBehavior::Queue => SendDecision::Enqueue,
            FollowUpBehavior::Steer => SendDecision::Steer,
        };
    }
    SendDecision::Prompt
}

impl State {
    /// The settings document's follow-up behaviour; `queue` before the document loads.
    pub fn follow_up_behavior(&self) -> FollowUpBehavior {
        self.settings_document
            .value
            .as_ref()
            .map_or(FollowUpBehavior::Queue, Settings::follow_up_behavior)
    }

    /// The decision inputs the composer and the open session give for `key`.
    pub(crate) fn send_input(&self, key: SendKey) -> SendInput {
        let open = self.open.as_ref().map(|open| open.id.clone());
        SendInput {
            text: self.composer.draft.clone(),
            idle: self.open_status() == SessionStatus::Idle,
            follow_up: self.follow_up_behavior(),
            key,
            first_send_pending: self.composer.status == ComposerStatus::Sending,
            has_queue: open
                .as_ref()
                .is_some_and(|session| !self.queue_items(session).is_empty()),
        }
    }
}

#[cfg(test)]
#[path = "send_test.rs"]
mod send_test;
