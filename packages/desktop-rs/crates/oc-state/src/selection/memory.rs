//! The per-session memory the agent switch reads (`contextStore.ts:88-205`).
//!
//! The model last used with each agent, and the Thinking level last chosen per agent and
//! model, live with the session. Switching back to an agent restores the model it ran on.

use std::collections::BTreeMap;

use super::value::{EffortChoice, ModelKey};

/// One session's remembered agent choices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SelectionMemory {
    /// The agent the session was last switched to (`saveSessionAgentSelection`).
    pub(crate) agent: Option<String>,
    /// The model last used with each agent (`saveAgentModelForSession`).
    pub(crate) agent_models: BTreeMap<String, ModelKey>,
    /// The explicit Thinking level last chosen per agent and model
    /// (`saveAgentModelVariantForSession`); Default and named levels only.
    pub(crate) agent_variants: BTreeMap<(String, ModelKey), EffortChoice>,
}

impl SelectionMemory {
    /// The model remembered for `agent`.
    pub(crate) fn agent_model(&self, agent: &str) -> Option<&ModelKey> {
        self.agent_models.get(agent)
    }

    /// The explicit Thinking level remembered for `agent` and `key`.
    pub(crate) fn agent_variant(&self, agent: &str, key: &ModelKey) -> Option<&EffortChoice> {
        self.agent_variants.get(&(agent.to_owned(), key.clone()))
    }

    /// Remembers that `agent` last used `key`.
    pub(crate) fn remember_model(&mut self, agent: &str, key: &ModelKey) {
        self.agent_models.insert(agent.to_owned(), key.clone());
    }

    /// Remembers an explicit Thinking level; no choice forgets the entry.
    pub(crate) fn remember_variant(
        &mut self,
        agent: &str,
        key: &ModelKey,
        choice: Option<&EffortChoice>,
    ) {
        let entry = (agent.to_owned(), key.clone());
        match choice {
            Some(choice) => {
                self.agent_variants.insert(entry, choice.clone());
            }
            None => {
                self.agent_variants.remove(&entry);
            }
        }
    }
}

#[cfg(test)]
#[path = "memory_test.rs"]
mod memory_test;
