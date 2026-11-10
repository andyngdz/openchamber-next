//! Messages of the open session as the chat column draws them.
//!
//! One assistant message per step, as on the wire: an agent turn with 20 tool calls is 21
//! assistant messages. Grouping steps into a turn is a layout concern.

use oc_client::types::{ModelRef, ToolStatus};
use serde_json::Value;

use crate::ids::{CallId, MessageId};

/// One message of the chat column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatMessage {
    /// What the user sent, aligned right.
    User(UserMessage),
    /// One step of the assistant's reply.
    Assistant(AssistantMessage),
}

impl ChatMessage {
    /// The message id, shared by both kinds.
    pub fn id(&self) -> &MessageId {
        match self {
            Self::User(user) => &user.id,
            Self::Assistant(assistant) => &assistant.id,
        }
    }
}

/// Whether a user message has reached the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// In the session's inbox, not yet handed to the model.
    Queued,
    /// Handed to the model.
    Delivered,
}

/// A user message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessage {
    /// The client-minted `msg_…` id.
    pub id: MessageId,
    /// What the user typed.
    pub text: String,
    /// Inbox state.
    pub delivery: Delivery,
}

/// Start and end of one step, in milliseconds since the epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepTiming {
    /// The step is still streaming.
    Running {
        /// When the step started.
        started: u64,
    },
    /// The step ended.
    Completed {
        /// When the step started.
        started: u64,
        /// When the step ended.
        completed: u64,
    },
}

impl StepTiming {
    /// Duration of a finished step, for the meta row.
    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            Self::Running { .. } => None,
            Self::Completed { started, completed } => Some(completed.saturating_sub(*started)),
        }
    }
}

/// One step of the assistant's reply, with what its meta row shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantMessage {
    /// The step's `assistantMessageID`.
    pub id: MessageId,
    /// Agent that ran the step.
    pub agent: String,
    /// Model the step ran on.
    pub model: ModelRef,
    /// Text and tool items in the order they arrived.
    pub items: Vec<Item>,
    /// Start and end of the step.
    pub timing: StepTiming,
}

impl AssistantMessage {
    fn text_mut(&mut self, ordinal: u32) -> Option<&mut TextItem> {
        self.items.iter_mut().find_map(|item| match item {
            Item::Text(text) if text.ordinal == ordinal => Some(text),
            Item::Text(_) | Item::Tool(_) => None,
        })
    }

    fn push_text(&mut self, ordinal: u32, text: &str) {
        self.items.push(Item::Text(TextItem {
            ordinal,
            text: text.to_owned(),
        }));
    }

    /// Adds an empty text item unless one with this ordinal exists; a delta may have created it.
    pub(crate) fn start_text(&mut self, ordinal: u32) {
        if self.text_mut(ordinal).is_none() {
            self.push_text(ordinal, "");
        }
    }

    /// Appends to the text item, creating it when its start was missed.
    pub(crate) fn append_text(&mut self, ordinal: u32, delta: &str) {
        match self.text_mut(ordinal) {
            Some(item) => item.text.push_str(delta),
            None => self.push_text(ordinal, delta),
        }
    }

    /// Replaces the text item with its full text, creating it when needed.
    pub(crate) fn replace_text(&mut self, ordinal: u32, text: &str) {
        match self.text_mut(ordinal) {
            Some(item) => text.clone_into(&mut item.text),
            None => self.push_text(ordinal, text),
        }
    }

    /// The tool row of this call.
    pub(crate) fn tool_mut(&mut self, call_id: &str) -> Option<&mut ToolItem> {
        self.items.iter_mut().find_map(|item| match item {
            Item::Tool(tool) if tool.call_id == *call_id => Some(tool),
            Item::Tool(_) | Item::Text(_) => None,
        })
    }
}

/// One item of an assistant message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// A text block, drawn as plain text.
    Text(TextItem),
    /// A tool call, drawn as its title row under the Activity row.
    Tool(ToolItem),
}

/// A text block, identified inside its message by its ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextItem {
    /// Position among the message's text blocks, from 0.
    pub ordinal: u32,
    /// The text so far.
    pub text: String,
}

/// A tool call row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolItem {
    /// Call id.
    pub call_id: CallId,
    /// Tool name.
    pub name: String,
    /// The tool's arguments; `Null` until `session.tool.called` delivers them.
    pub input: Value,
    /// Where the call is in its life.
    pub status: ToolStatus,
}
