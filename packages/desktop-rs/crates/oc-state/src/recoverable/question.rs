//! The question classification and the link a question form keeps to its tool call
//! (`sync/recoverable-questions.ts:42-67`).

use oc_client::form::FormInfo;
use serde_json::Value;

use crate::interrupted::LOCAL_INTERRUPTED;
use crate::requests::METADATA_KIND;

/// How one question's ask is judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionAskOutcome {
    /// The tool completed or the user replied.
    Answered,
    /// The ask was deliberately dismissed.
    Dismissed,
    /// The turn died while the ask waited.
    TurnDeath,
    /// OpenCode itself shut down; the turn stays resumable.
    Shutdown,
    /// The tool call still waits.
    Pending,
    /// Evidence this client does not recognize.
    Unresolved,
}

/// Why a question was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedOutcome {
    /// Answered by the user.
    Answered,
    /// Dismissed by the user.
    Dismissed,
}

/// How the loaded history recorded the end of a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedOutcome {
    /// The turn finished.
    Succeeded,
    /// The turn was interrupted.
    Interrupted,
    /// The turn failed.
    Failed,
    /// An outcome this client does not know.
    Other,
}

/// What proves the question's ask died.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryEvidence {
    /// OpenCode reported the tool call interrupted.
    ToolInterrupted,
    /// This client marked the open tool part when the turn died.
    LocalInterruptMark,
}

/// Who wrote the tool's error: this client or OpenCode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorOrigin {
    /// This client manufactured the interruption.
    Local,
    /// OpenCode reported the failure.
    Server,
}

/// The tool call behind a question form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionToolLink {
    /// The assistant message that called the tool.
    pub message_id: String,
    /// The tool call id.
    pub tool_id: String,
}

/// A question offered again because its first ask died with the turn.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveredQuestion {
    /// The original form.
    pub form: FormInfo,
    /// The directory it was seen in.
    pub directory: String,
    /// The asking session.
    pub session_id: String,
    /// The assistant message that called the question tool.
    pub tool_message_id: String,
    /// What proves the turn died.
    pub evidence: RecoveryEvidence,
}

/// Unknown wording fails closed; shutdown keeps OpenCode's turn resumable
/// (`recoverable-questions.ts:52-59`).
pub fn classify_question_tool_error(
    error: &str,
    error_type: Option<&str>,
    origin: ErrorOrigin,
) -> QuestionAskOutcome {
    if origin == ErrorOrigin::Local {
        return if error == LOCAL_INTERRUPTED {
            QuestionAskOutcome::TurnDeath
        } else {
            QuestionAskOutcome::Unresolved
        };
    }
    if error_type.is_some_and(|kind| kind != "aborted") {
        return QuestionAskOutcome::Dismissed;
    }
    match error {
        "Tool execution interrupted" => QuestionAskOutcome::TurnDeath,
        "Interaction cancelled because the location shut down" => QuestionAskOutcome::Shutdown,
        _ => QuestionAskOutcome::Unresolved,
    }
}

/// The tool call a session-owned question form points at (`recoverable-questions.ts:42-50`).
pub fn question_tool_link(form: &FormInfo) -> Option<QuestionToolLink> {
    if form.session_id == crate::requests::GLOBAL_SESSION
        || metadata_str(form, METADATA_KIND) != Some("question")
    {
        return None;
    }
    let tool = form.metadata.get("tool")?.as_object()?;
    let message_id = tool.get("messageID")?.as_str()?;
    let tool_id = tool.get("id")?.as_str()?;
    if message_id.is_empty() || tool_id.is_empty() {
        return None;
    }
    Some(QuestionToolLink {
        message_id: message_id.to_owned(),
        tool_id: tool_id.to_owned(),
    })
}

fn metadata_str<'a>(form: &'a FormInfo, key: &str) -> Option<&'a str> {
    form.metadata.get(key).and_then(Value::as_str)
}

#[cfg(test)]
#[path = "question_test.rs"]
mod question_test;
