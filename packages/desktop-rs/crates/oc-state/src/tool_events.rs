//! Tool transitions that the recovered-question rule reads.
//!
//! A success or failure records the question's disposition and clears the local interrupt
//! mark; a later authoritative transition replaces the local mark (`event-reducer.ts:213`).

use oc_client::{Event, EventData};

use crate::recoverable::{QuestionAskOutcome, QuestionToolLink, Recoverable};

/// How a tool transition meets the part's current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolTransition {
    /// The part's existing terminal state wins.
    Authoritative,
    /// The transition replaces this client's local interrupt mark.
    ReplaceLocalMark,
}

/// Records one tool transition and says whether it replaces a local mark.
pub(crate) fn record(state: &mut Recoverable, event: &Event) -> ToolTransition {
    match &event.data {
        EventData::ToolSuccess(tool) => {
            let force = state
                .evidence_mut()
                .clear_tool(&tool.assistant_message_id, &tool.id);
            state.record_disposition(
                &tool.session_id,
                &link(&tool.assistant_message_id, &tool.id),
                QuestionAskOutcome::Answered,
            );
            if force {
                ToolTransition::ReplaceLocalMark
            } else {
                ToolTransition::Authoritative
            }
        }
        EventData::ToolFailed(tool) => {
            let force = state
                .evidence_mut()
                .clear_tool(&tool.assistant_message_id, &tool.id);
            let outcome = state.evidence_mut().record_failure(
                &tool.assistant_message_id,
                &tool.id,
                tool.error_message().unwrap_or_default(),
                tool.error_kind(),
            );
            state.record_disposition(
                &tool.session_id,
                &link(&tool.assistant_message_id, &tool.id),
                outcome,
            );
            if force {
                ToolTransition::ReplaceLocalMark
            } else {
                ToolTransition::Authoritative
            }
        }
        _ => ToolTransition::Authoritative,
    }
}

/// The link one tool call's disposition is recorded under.
pub(crate) fn link(message_id: &str, tool_id: &str) -> QuestionToolLink {
    QuestionToolLink {
        message_id: message_id.to_owned(),
        tool_id: tool_id.to_owned(),
    }
}

#[cfg(test)]
#[path = "tool_events_test.rs"]
mod tool_events_test;
