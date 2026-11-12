//! Questions whose ask died with its turn, offered again after the form leaves the store
//! (`sync/recoverable-questions.ts:40-205`, design "Recovered questions").
//!
//! A form is seen when it is a session-owned `question` form linked to a tool call. It is
//! offered once it leaves the pending store, when that call's outcome is a turn death or a
//! settlement recorded it. Submit and Dismiss close it for good, and the seen map keeps at
//! most 100 entries.

use std::collections::BTreeMap;

use oc_client::form::FormInfo;

use crate::interrupted::ToolEvidence;

mod offer;
mod question;

pub use question::{
    ClosedOutcome, ErrorOrigin, QuestionAskOutcome, QuestionToolLink, RecordedOutcome,
    RecoveredQuestion, RecoveryEvidence, classify_question_tool_error, question_tool_link,
};

/// One question form remembered with the tool call that asked it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SeenQuestion {
    form: FormInfo,
    directory: String,
    session_id: String,
    tool: QuestionToolLink,
}

/// The key one tool call's disposition is stored under.
pub(crate) fn tool_key(tool: &QuestionToolLink) -> String {
    format!("{}\u{0}{}", tool.message_id, tool.tool_id)
}

/// React's cap on remembered questions (`recoverable-questions.ts:34`).
const MAX_SEEN_QUESTIONS: usize = 100;

/// The recovery store: seen questions, their outcomes, and tool evidence.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Recoverable {
    seen: Vec<(String, SeenQuestion)>,
    outcomes: BTreeMap<String, ClosedOutcome>,
    dispositions: BTreeMap<String, QuestionAskOutcome>,
    evidence: ToolEvidence,
}

impl Recoverable {
    /// Records a form from `form.created`.
    pub(crate) fn note_form(&mut self, form: &FormInfo, directory: &str) {
        let Some(tool) = question_tool_link(form) else {
            return;
        };
        if self.seen.iter().any(|(id, _)| id == &form.id) {
            return;
        }
        self.seen.push((
            form.id.clone(),
            SeenQuestion {
                form: form.clone(),
                directory: directory.to_owned(),
                session_id: form.session_id.clone(),
                tool,
            },
        ));
        while self.seen.len() > MAX_SEEN_QUESTIONS {
            self.evict_oldest();
        }
    }

    /// Records every question form a pending list carried
    /// (`recoverable-questions.ts:122-124`).
    pub(crate) fn note_snapshot(&mut self, directory: &str, forms: &[FormInfo]) {
        for form in forms {
            self.note_form(form, directory);
        }
    }

    /// Closes a seen question; a question never seen has no tombstone to leave.
    pub(crate) fn close(&mut self, form_id: &str, outcome: ClosedOutcome) {
        if !self.seen.iter().any(|(id, _)| id == form_id) || self.outcomes.contains_key(form_id) {
            return;
        }
        self.outcomes.insert(form_id.to_owned(), outcome);
    }

    /// The user replied to the question.
    pub(crate) fn note_replied(&mut self, form_id: &str) {
        self.close(form_id, ClosedOutcome::Answered);
    }

    /// The user dismissed the question.
    pub(crate) fn note_cancelled(&mut self, form_id: &str) {
        self.close(form_id, ClosedOutcome::Dismissed);
    }

    /// Records a tool transition for a seen question's own call; an earlier settlement wins.
    pub(crate) fn record_disposition(
        &mut self,
        session: &str,
        tool: &QuestionToolLink,
        outcome: QuestionAskOutcome,
    ) {
        let key = tool_key(tool);
        let belongs = self
            .seen
            .iter()
            .any(|(_, seen)| seen.session_id == session && tool_key(&seen.tool) == key);
        if !belongs {
            return;
        }
        let previous = self.dispositions.get(&key).copied();
        if previous == Some(outcome)
            || matches!(
                previous,
                Some(
                    QuestionAskOutcome::Answered
                        | QuestionAskOutcome::Dismissed
                        | QuestionAskOutcome::Shutdown
                )
            )
        {
            return;
        }
        self.dispositions.insert(key, outcome);
    }

    /// Drops every question of a deleted session.
    pub(crate) fn drop_session(&mut self, session: &str) {
        self.seen.retain(|(_, seen)| seen.session_id != session);
        self.outcomes
            .retain(|id, _| self.seen.iter().any(|(seen_id, _)| seen_id == id));
        self.dispositions.retain(|key, _| {
            self.seen
                .iter()
                .any(|(_, seen)| tool_key(&seen.tool) == *key)
        });
    }

    /// The tool evidence the interrupt mark and tool events maintain.
    pub(crate) fn evidence_mut(&mut self) -> &mut ToolEvidence {
        &mut self.evidence
    }

    /// The tool evidence, read-only.
    #[cfg(test)]
    pub(crate) fn evidence(&self) -> &ToolEvidence {
        &self.evidence
    }
}

impl crate::state::State {
    /// Questions whose ask died, outside the pending store, for the open session's tree.
    pub fn recovered_questions(&self, session: &crate::ids::SessionId) -> Vec<RecoveredQuestion> {
        let scope = self.subtree_ids(session.as_str());
        let mut pending_forms = std::collections::BTreeSet::new();
        for scoped in &scope {
            for form in self.requests.forms_for(scoped) {
                pending_forms.insert(form.id.clone());
            }
        }
        let messages: &[crate::message::ChatMessage] = self
            .open
            .as_ref()
            .filter(|open| open.id == *session.as_str())
            .map_or(&[], |open| open.messages.as_slice());
        self.recoverable.offer(messages, &scope, &pending_forms)
    }
}

#[cfg(test)]
#[path = "recoverable_test.rs"]
mod recoverable_test;
