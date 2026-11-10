//! The open session's messages grouped the way the chat column draws them: one turn per user
//! message, its steps' text in order, runs of tool calls under one Activity row, and a meta
//! row with model, agent and duration once the turn's last step has ended.

use oc_state::{AssistantMessage, ChatMessage, Item, StepTiming, ToolItem, ToolStatus};

/// A user message and the assistant steps that answer it.
#[derive(Debug, Default)]
pub(crate) struct Turn<'a> {
    pub(crate) user_text: Option<&'a str>,
    pub(crate) steps: Vec<&'a AssistantMessage>,
}

/// One block of a turn's reply.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Block<'a> {
    /// A non-empty text item.
    Text(&'a str),
    /// Consecutive tool calls.
    Activity(Vec<&'a ToolItem>),
}

/// What the meta row under a finished turn shows.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TurnMeta {
    pub(crate) model: String,
    pub(crate) agent: String,
    pub(crate) duration: String,
}

/// Splits the messages into turns; steps before the first user message form a turn of their own.
pub(crate) fn turns(messages: &[ChatMessage]) -> Vec<Turn<'_>> {
    let mut turns: Vec<Turn<'_>> = Vec::new();
    for message in messages {
        match message {
            ChatMessage::User(user) => turns.push(Turn {
                user_text: Some(&user.text),
                steps: Vec::new(),
            }),
            ChatMessage::Assistant(step) => match turns.last_mut() {
                Some(turn) => turn.steps.push(step),
                None => turns.push(Turn {
                    user_text: None,
                    steps: vec![step],
                }),
            },
        }
    }
    turns
}

impl<'a> Turn<'a> {
    /// Text and Activity blocks in arrival order.
    pub(crate) fn blocks(&self) -> Vec<Block<'a>> {
        let mut blocks: Vec<Block<'a>> = Vec::new();
        for item in self.steps.iter().flat_map(|step| step.items.iter()) {
            match item {
                Item::Text(text) if text.text.trim().is_empty() => {}
                Item::Text(text) => blocks.push(Block::Text(text.text.trim_end())),
                Item::Tool(tool) => match blocks.last_mut() {
                    Some(Block::Activity(tools)) => tools.push(tool),
                    Some(Block::Text(_)) | None => blocks.push(Block::Activity(vec![tool])),
                },
            }
        }
        blocks
    }

    /// The meta row, once the last step has ended.
    pub(crate) fn meta(&self) -> Option<TurnMeta> {
        let first = self.steps.first()?;
        let last = self.steps.last()?;
        let StepTiming::Completed { completed, .. } = last.timing else {
            return None;
        };
        let started = match first.timing {
            StepTiming::Running { started } | StepTiming::Completed { started, .. } => started,
        };
        Some(TurnMeta {
            model: last.model.id.clone(),
            agent: last.agent.clone(),
            duration: format_seconds(completed.saturating_sub(started)),
        })
    }
}

/// `24.5s` for 24 512 ms.
pub(crate) fn format_seconds(milliseconds: u64) -> String {
    let tenths = (milliseconds + 50) / 100;
    format!("{}.{}s", tenths / 10, tenths % 10)
}

/// `glob: completed`.
pub(crate) fn tool_line(tool: &ToolItem) -> String {
    let status = match tool.status {
        ToolStatus::Pending => "pending",
        ToolStatus::Running => "running",
        ToolStatus::Completed => "completed",
        ToolStatus::Error => "error",
        ToolStatus::Other => "unknown",
    };
    format!("{}: {status}", tool.name)
}

#[cfg(test)]
#[path = "turns_test.rs"]
mod turns_test;
