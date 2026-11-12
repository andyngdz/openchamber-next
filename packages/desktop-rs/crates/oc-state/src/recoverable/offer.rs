//! The offer rule: which seen questions leave the pending store as recovered questions
//! (`recoverable-questions.ts:75-99`).

use std::collections::BTreeSet;

use crate::message::ChatMessage;
use crate::recoverable::{
    QuestionAskOutcome, Recoverable, RecoveredQuestion, RecoveryEvidence, tool_key,
};

impl Recoverable {
    /// Questions that left the pending store and whose ask died.
    pub(crate) fn offer(
        &self,
        messages: &[ChatMessage],
        scope: &[String],
        pending_forms: &BTreeSet<String>,
    ) -> Vec<RecoveredQuestion> {
        let mut recovered = Vec::new();
        for (form_id, seen) in &self.seen {
            if self.outcomes.contains_key(form_id)
                || !scope.iter().any(|id| id == &seen.session_id)
                || pending_forms.contains(form_id)
            {
                continue;
            }
            let key = tool_key(&seen.tool);
            let outcome = self.dispositions.get(&key).copied().unwrap_or_else(|| {
                self.evidence
                    .outcome_of(messages, &seen.tool.message_id, &seen.tool.tool_id)
            });
            if outcome != QuestionAskOutcome::TurnDeath {
                continue;
            }
            let evidence = if self
                .evidence
                .is_local(&seen.tool.message_id, &seen.tool.tool_id)
            {
                RecoveryEvidence::LocalInterruptMark
            } else {
                RecoveryEvidence::ToolInterrupted
            };
            recovered.push(RecoveredQuestion {
                form: seen.form.clone(),
                directory: seen.directory.clone(),
                session_id: seen.session_id.clone(),
                tool_message_id: seen.tool.message_id.clone(),
                evidence,
            });
        }
        recovered
    }

    pub(crate) fn evict_oldest(&mut self) {
        if self.seen.is_empty() {
            return;
        }
        let (form_id, evicted) = self.seen.remove(0);
        self.outcomes.remove(&form_id);
        let key = tool_key(&evicted.tool);
        if !self
            .seen
            .iter()
            .any(|(_, seen)| tool_key(&seen.tool) == key)
        {
            self.dispositions.remove(&key);
        }
    }

    /// How many questions are remembered.
    #[cfg(test)]
    pub(crate) fn seen_len(&self) -> usize {
        self.seen.len()
    }

    /// How many questions are closed.
    #[cfg(test)]
    pub(crate) fn outcome_len(&self) -> usize {
        self.outcomes.len()
    }

    /// How many tool dispositions are held.
    #[cfg(test)]
    pub(crate) fn disposition_len(&self) -> usize {
        self.dispositions.len()
    }

    /// Whether this question is remembered.
    #[cfg(test)]
    pub(crate) fn has_seen(&self, form_id: &str) -> bool {
        self.seen.iter().any(|(id, _)| id == form_id)
    }

    /// Whether this question is closed.
    #[cfg(test)]
    pub(crate) fn has_outcome(&self, form_id: &str) -> bool {
        self.outcomes.contains_key(form_id)
    }

    /// Whether this tool call has a disposition.
    #[cfg(test)]
    pub(crate) fn has_disposition(&self, tool: &crate::recoverable::QuestionToolLink) -> bool {
        self.dispositions.contains_key(&tool_key(tool))
    }
}
