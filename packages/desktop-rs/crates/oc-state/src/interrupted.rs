//! The local `Interrupted` mark and the tool evidence recovered questions read.
//!
//! When a turn is stopped or fails, every open tool part of its trailing assistant message
//! becomes `error` with a local `Interrupted` mark (`sync-context.tsx:2695-2810`). A later
//! authoritative tool transition replaces the mark (`event-reducer.ts:213`).

use std::collections::{BTreeMap, BTreeSet};

use oc_client::types::{AssistantContent, Message, ToolErrorSource, ToolStatus};

use crate::message::{ChatMessage, Item};
use crate::recoverable::{
    ErrorOrigin, QuestionAskOutcome, RecordedOutcome, classify_question_tool_error,
};

/// The wording this client writes on a tool part it marked interrupted.
pub(crate) const LOCAL_INTERRUPTED: &str = "Interrupted";

/// The error a failed tool reported, kept for the question classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolError {
    /// The tool's error message.
    pub(crate) message: String,
    /// The server's `errorType`, when it named one.
    pub(crate) error_type: Option<String>,
}

/// What the state learned about tool calls, beyond what a message list carries.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ToolEvidence {
    errors: BTreeMap<(String, String), ToolError>,
    local_marks: BTreeSet<(String, String)>,
}

fn key(message_id: &str, call_id: &str) -> (String, String) {
    (message_id.to_owned(), call_id.to_owned())
}

impl ToolEvidence {
    /// Reads a loaded history's tool errors and, when that history records the turn's
    /// death, the local `Interrupted` marks React's `markRecordedInterruptedTurn` writes.
    /// Returns the open parts whose status becomes `error`.
    pub(crate) fn ingest_loaded(
        &mut self,
        messages: &[Message],
        recorded: Option<RecordedOutcome>,
    ) -> Vec<(String, String)> {
        let mut loaded = BTreeSet::new();
        for message in messages {
            let Message::Assistant { id, content, .. } = message else {
                continue;
            };
            for entry in content {
                if let AssistantContent::Tool { id: call, .. } = entry {
                    loaded.insert(key(id, call));
                }
            }
        }
        self.errors.retain(|held, _| !loaded.contains(held));
        self.local_marks.retain(|held| !loaded.contains(held));
        for message in messages {
            let Message::Assistant { id, content, .. } = message else {
                continue;
            };
            for entry in content {
                let AssistantContent::Tool {
                    id: call, state, ..
                } = entry
                else {
                    continue;
                };
                if let Some(message) = state.error_message() {
                    self.errors.insert(
                        key(id, call),
                        ToolError {
                            message: message.to_owned(),
                            error_type: state.error_kind().map(str::to_owned),
                        },
                    );
                }
                if state.error_source == Some(ToolErrorSource::Local) {
                    self.local_marks.insert(key(id, call));
                }
            }
        }
        let mut promoted = Vec::new();
        if !matches!(
            recorded,
            Some(RecordedOutcome::Interrupted | RecordedOutcome::Failed)
        ) {
            return promoted;
        }
        let Some(Message::Assistant { id, content, .. }) = crate::load::trailing_loaded(messages)
        else {
            return promoted;
        };
        for entry in content {
            let AssistantContent::Tool {
                id: call, state, ..
            } = entry
            else {
                continue;
            };
            let key = key(id, call);
            let marked = state.status == ToolStatus::Error
                && state.error_message() == Some(LOCAL_INTERRUPTED)
                && state.error_source.is_none()
                && state.error_kind().is_none();
            if marked {
                self.local_marks.insert(key.clone());
            }
            if matches!(state.status, ToolStatus::Pending | ToolStatus::Running) {
                self.local_marks.insert(key.clone());
                promoted.push(key);
            }
        }
        promoted
    }

    /// Stores a failed tool's error and classifies it as the server reported it.
    pub(crate) fn record_failure(
        &mut self,
        message_id: &str,
        call_id: &str,
        message: &str,
        error_type: Option<&str>,
    ) -> QuestionAskOutcome {
        self.errors.insert(
            key(message_id, call_id),
            ToolError {
                message: message.to_owned(),
                error_type: error_type.map(str::to_owned),
            },
        );
        classify_question_tool_error(message, error_type, ErrorOrigin::Server)
    }

    /// Removes the local mark and any stored error; `true` when a local mark was there.
    pub(crate) fn clear_tool(&mut self, message_id: &str, call_id: &str) -> bool {
        let key = key(message_id, call_id);
        self.errors.remove(&key);
        self.local_marks.remove(&key)
    }

    /// Marks every pending or running tool part of the trailing assistant message.
    pub(crate) fn mark_trailing(&mut self, messages: &mut [ChatMessage]) {
        let Some(step) = trailing_assistant(messages) else {
            return;
        };
        let message_id = step.id.to_string();
        for item in &mut step.items {
            let Item::Tool(tool) = item else {
                continue;
            };
            if !matches!(tool.status, ToolStatus::Pending | ToolStatus::Running) {
                continue;
            }
            tool.status = ToolStatus::Error;
            let key = key(&message_id, tool.call_id.as_str());
            self.errors.remove(&key);
            self.local_marks.insert(key);
        }
    }

    /// The outcome of one question's tool call.
    pub(crate) fn outcome_of(
        &self,
        messages: &[ChatMessage],
        message_id: &str,
        call_id: &str,
    ) -> QuestionAskOutcome {
        let Some(tool) = find_tool(messages, message_id, call_id) else {
            return QuestionAskOutcome::Unresolved;
        };
        let key = key(message_id, call_id);
        match tool.status {
            ToolStatus::Completed => QuestionAskOutcome::Answered,
            ToolStatus::Error if self.local_marks.contains(&key) => {
                classify_question_tool_error(LOCAL_INTERRUPTED, None, ErrorOrigin::Local)
            }
            ToolStatus::Error => match self.errors.get(&key) {
                Some(error) => classify_question_tool_error(
                    &error.message,
                    error.error_type.as_deref(),
                    ErrorOrigin::Server,
                ),
                None => QuestionAskOutcome::Unresolved,
            },
            ToolStatus::Pending | ToolStatus::Running | ToolStatus::Other => {
                QuestionAskOutcome::Pending
            }
        }
    }

    /// Whether the part carries this client's local `Interrupted` mark.
    pub(crate) fn is_local(&self, message_id: &str, call_id: &str) -> bool {
        self.local_marks.contains(&key(message_id, call_id))
    }
}

fn trailing_assistant(
    messages: &mut [ChatMessage],
) -> Option<&mut crate::message::AssistantMessage> {
    let (index, message) = messages.iter().enumerate().next_back()?;
    if matches!(message, ChatMessage::User(_)) {
        return None;
    }
    match messages.get_mut(index) {
        Some(ChatMessage::Assistant(step)) => Some(step),
        _ => None,
    }
}

fn find_tool<'a>(
    messages: &'a [ChatMessage],
    message_id: &str,
    call_id: &str,
) -> Option<&'a crate::message::ToolItem> {
    let step = messages.iter().find_map(|message| match message {
        ChatMessage::Assistant(step) if step.id == *message_id => Some(step),
        ChatMessage::Assistant(_) | ChatMessage::User(_) => None,
    })?;
    step.items.iter().find_map(|item| match item {
        Item::Tool(tool) if tool.call_id == *call_id => Some(tool),
        Item::Tool(_) | Item::Text(_) => None,
    })
}

impl crate::state::State {
    /// Marks every open tool part of the session's trailing message after its turn died.
    pub(crate) fn mark_interrupted_turn(&mut self, session: &str) {
        let crate::state::State {
            open, recoverable, ..
        } = self;
        let Some(open) = open.as_mut().filter(|open| open.id == *session) else {
            return;
        };
        recoverable.evidence_mut().mark_trailing(&mut open.messages);
    }
}

#[cfg(test)]
#[path = "interrupted_test.rs"]
mod interrupted_test;
