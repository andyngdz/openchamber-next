//! The prompt body and its accepted answer (`POST /api/session/:id/prompt`).
//!
//! The plain composer send carries only `id` and `text`; a steer adds `delivery: "steer"`
//! (`packages/ui/src/lib/opencode/client.ts:1610-1620`).

use serde::{Deserialize, Serialize};

/// How the server delivers a prompt that arrives during a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    /// Steer the text into the running turn.
    Steer,
    /// Queue the text until the turn ends.
    Queue,
}

/// Body of `POST /api/session/:id/prompt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PromptRequest {
    /// Client-minted `msg_…` id, see [`crate::new_message_id`].
    pub id: String,
    /// What the user typed.
    pub text: String,
    /// `steer` for a message sent into a running turn; absent for a plain send.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
}

/// The accepted prompt, echoed back by the server as the user message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PromptAccepted {
    /// The `msg_…` id the client sent.
    pub id: String,
    /// Session the prompt went to.
    #[serde(rename = "sessionID", default)]
    pub session_id: String,
    /// The delivery the server admitted the message with; absent on older servers.
    #[serde(default)]
    pub delivery: Option<Delivery>,
}

#[cfg(test)]
#[path = "prompt_test.rs"]
mod prompt_test;
