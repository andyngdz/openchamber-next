//! What a pick does to the selection and the settings it writes.
//!
//! A model pick puts the model in Recent, records `lastSelectedModel` and remembers it for
//! the session's agent (`ModelControls.tsx:1012-1023, 1613-1645`). An agent pick prepends a
//! new agent to `recentAgents` and switches the model through [`resolve_agent_selection`].
//! A variant pick and a successful send write `lastUsedEfforts`; favorites toggle.

use super::access::project_defaults;
use super::memory::SelectionMemory;
use super::patches::{
    add_recent_agent, add_recent_model, merge_patch, remember_last_selected_model,
    remember_last_used_effort, toggle_favorite_agent, toggle_favorite_model,
};
use super::set_agent::{AgentContext, SessionScope, resolve_agent_selection};
use super::value::{EffortChoice, ModelKey, ModelPick, SelectionSource};
use super::variant::inherited_variant;
use crate::ids::SessionId;
use crate::state::State;

impl State {
    pub(super) fn pick_agent(&mut self, agent_name: &str) {
        let session = self.open_session_id();
        let scope = match session {
            Some(_) => SessionScope::Session,
            None => SessionScope::Draft,
        };
        let current = self.current_selection().cloned().unwrap_or_default();
        let directory = self.selection_directory();
        let outcome = {
            let Some(settings) = self.settings_document.value.as_ref() else {
                return;
            };
            let project = directory
                .as_deref()
                .map(|directory| project_defaults(settings, directory))
                .unwrap_or_default();
            let catalog = directory
                .as_deref()
                .and_then(|directory| self.catalog(directory));
            let memory = session
                .as_ref()
                .and_then(|id| self.selection.memories.get(id))
                .cloned()
                .unwrap_or_default();
            let context = AgentContext {
                settings,
                project,
                catalog,
                memory: &memory,
                scope,
            };
            resolve_agent_selection(context, &current, agent_name)
        };
        let same_model = self.selection_slot_mut().model.as_ref() == outcome.model.as_ref();
        let slot = self.selection_slot_mut();
        slot.agent = Some(agent_name.to_owned());
        slot.agent_source = SelectionSource::Manual;
        slot.model = outcome.model.clone();
        slot.variant = outcome.variant.clone();
        slot.variant_override = outcome.variant_override.clone();
        slot.model_source = outcome.model_source;
        if !same_model {
            slot.model_pick = ModelPick::None;
        }
        self.remember_agent_pick(session, agent_name, &outcome);
        let patch = {
            let Some(settings) = self.settings_document.value.as_mut() else {
                return;
            };
            add_recent_agent(settings, agent_name)
        };
        self.queue_settings_patch(patch);
    }

    pub(super) fn pick_model(&mut self, provider_id: &str, model_id: &str) {
        let key = ModelKey::new(provider_id, model_id);
        let directory = self.selection_directory();
        let Some(catalog) = directory
            .as_deref()
            .and_then(|directory| self.catalog(directory))
        else {
            return;
        };
        if !catalog.has_model(&key) {
            return;
        }
        let current = self.current_selection().cloned().unwrap_or_default();
        let variant = {
            let Some(settings) = self.settings_document.value.as_ref() else {
                return;
            };
            let project = directory
                .as_deref()
                .map(|directory| project_defaults(settings, directory))
                .unwrap_or_default();
            let memory = SelectionMemory::default();
            let context = AgentContext {
                settings,
                project,
                catalog: Some(catalog),
                memory: &memory,
                scope: SessionScope::Session,
            };
            let agent = current.agent().and_then(|name| catalog.agent(name));
            inherited_variant(context, agent, &key)
        };
        let slot = self.selection_slot_mut();
        slot.model = Some(key.clone());
        slot.variant = variant;
        slot.variant_override = None;
        slot.model_source = SelectionSource::Manual;
        slot.model_pick = ModelPick::Live(key.clone());
        if let (Some(session), Some(agent)) = (self.open_session_id(), current.agent()) {
            self.remember_model(session, agent, &key);
        }
        let patch = {
            let Some(settings) = self.settings_document.value.as_mut() else {
                return;
            };
            let mut patch = add_recent_model(settings, &key);
            merge_patch(&mut patch, remember_last_selected_model(settings, &key));
            patch
        };
        self.queue_settings_patch(patch);
    }

    pub(super) fn pick_variant(&mut self, variant: Option<&str>) {
        let current = self.current_selection().cloned().unwrap_or_default();
        let Some(key) = current.model().cloned() else {
            return;
        };
        let choice = variant
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map_or(EffortChoice::Default, |value| {
                EffortChoice::Named(value.to_owned())
            });
        let slot = self.selection_slot_mut();
        slot.variant = Some(choice.clone());
        slot.variant_override = Some(choice.clone());
        if let (Some(session), Some(agent)) = (self.open_session_id(), current.agent()) {
            let memory = self.selection.memories.entry(session).or_default();
            memory.remember_variant(agent, &key, Some(&choice));
        }
        let patch = {
            let Some(settings) = self.settings_document.value.as_mut() else {
                return;
            };
            remember_last_used_effort(settings, &key, choice.variant())
        };
        self.queue_settings_patch(patch);
    }

    pub(super) fn send_succeeded(&mut self) {
        let current = self.current_selection().cloned().unwrap_or_default();
        let patch = {
            let (Some(settings), Some(key)) =
                (self.settings_document.value.as_mut(), current.model())
            else {
                return;
            };
            let variant = current.variant().and_then(|choice| choice.variant());
            remember_last_used_effort(settings, key, variant)
        };
        self.queue_settings_patch(patch);
        let open = self.open.is_some();
        let slot = self.selection_slot_mut();
        slot.model_pick = ModelPick::None;
        if open {
            slot.agent_source = SelectionSource::Session;
            slot.model_source = SelectionSource::Session;
        }
    }

    pub(super) fn toggle_favorite_model(&mut self, provider_id: &str, model_id: &str) {
        let key = ModelKey::new(provider_id, model_id);
        let patch = {
            let Some(settings) = self.settings_document.value.as_mut() else {
                return;
            };
            toggle_favorite_model(settings, &key)
        };
        self.queue_settings_patch(patch);
    }

    pub(super) fn toggle_favorite_agent(&mut self, agent: &str) {
        let patch = {
            let Some(settings) = self.settings_document.value.as_mut() else {
                return;
            };
            toggle_favorite_agent(settings, agent)
        };
        self.queue_settings_patch(patch);
    }

    fn remember_model(&mut self, session: SessionId, agent: &str, key: &ModelKey) {
        let memory = self.selection.memories.entry(session).or_default();
        memory.remember_model(agent, key);
    }

    fn remember_agent_pick(
        &mut self,
        session: Option<SessionId>,
        agent_name: &str,
        outcome: &super::set_agent::AgentOutcome,
    ) {
        let Some(session) = session else {
            return;
        };
        let memory = self.selection.memories.entry(session).or_default();
        memory.agent = Some(agent_name.to_owned());
        if let (true, Some(key)) = (outcome.remember_model, outcome.model.as_ref()) {
            memory.remember_model(agent_name, key);
        }
    }
}

#[cfg(test)]
#[path = "picks_test.rs"]
mod picks_test;
