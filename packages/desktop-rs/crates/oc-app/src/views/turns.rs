//! The open session's messages grouped the way the chat column draws them: one turn per user
//! message, its steps' text in order, runs of tool calls under one Activity row, and a meta
//! row with model, agent and duration once the turn's last step has ended.

use std::collections::HashMap;

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

/// What a tool row shows: the tool's display name and the description React derives from the
/// input (`toolHelpers.ts` `getToolMetadata`, `tools.ts` `toolDescription`). Only `glob`
/// occurs in the fixtures, so only its title is ported; any other tool shows its raw name.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ToolTitle<'a> {
    pub(crate) name: &'a str,
    pub(crate) detail: Option<&'a str>,
}

impl<'a> ToolTitle<'a> {
    pub(crate) fn of(tool: &'a ToolItem) -> Self {
        match tool.name.as_str() {
            "glob" => Self {
                name: "Find Files",
                detail: tool.input["pattern"]
                    .as_str()
                    .filter(|text| !text.is_empty()),
            },
            other => Self {
                name: other,
                detail: None,
            },
        }
    }

    /// The accessible name: React's row button reads its name and its description.
    pub(crate) fn accessible_name(&self) -> String {
        match self.detail {
            Some(detail) => format!("{} {detail}", self.name),
            None => self.name.to_owned(),
        }
    }
}

/// What the row adds after its title while the call is not done; a finished call says nothing.
pub(crate) fn tool_status(tool: &ToolItem) -> Option<&'static str> {
    match tool.status {
        ToolStatus::Pending => Some("pending"),
        ToolStatus::Running => Some("running"),
        ToolStatus::Completed => None,
        ToolStatus::Error => Some("failed"),
        ToolStatus::Other => Some("unknown"),
    }
}

/// The `#n` index of each title in a region, from 1, or `None` for a title that occurs once:
/// the reference key repeats only where the name does.
pub(crate) fn repeat_indexes(titles: &[String]) -> Vec<Option<usize>> {
    let mut totals: HashMap<&str, usize> = HashMap::new();
    for title in titles {
        *totals.entry(title).or_default() += 1;
    }
    let mut seen: HashMap<&str, usize> = HashMap::new();
    titles
        .iter()
        .map(|title| {
            let index = seen.entry(title).or_default();
            *index += 1;
            (totals[title.as_str()] > 1).then_some(*index)
        })
        .collect()
}

#[cfg(test)]
#[path = "turns_test.rs"]
mod turns_test;
