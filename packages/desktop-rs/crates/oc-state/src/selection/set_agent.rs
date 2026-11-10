//! Picking an agent, ported from `setAgent` (`useConfigStore.ts:3475-3620`).
//!
//! An unsent model pick outranks everything. Otherwise the model remembered for the agent
//! is restored, then the agent's pinned model, then a manual model this agent does not
//! pin, then the settings default. The Thinking level is resolved by [`super::variant`].

use oc_client::catalog::Agent;
use oc_client::settings::Settings;

use super::cascade::ProjectDefaults;
use super::catalog::Catalog;
use super::memory::SelectionMemory;
use super::value::{EffortChoice, ModelKey, ModelPick, Selection, SelectionSource};
use super::variant::resolved_variant;

/// Which selection a pick changes: the draft's, or an open session's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionScope {
    /// A new Chat or project draft.
    Draft,
    /// An open session.
    Session,
}

/// The inputs every step of the agent switch reads.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AgentContext<'a> {
    /// The shared settings.
    pub(crate) settings: &'a Settings,
    /// The project's own defaults.
    pub(crate) project: ProjectDefaults<'a>,
    /// The directory's catalog, once loaded.
    pub(crate) catalog: Option<&'a Catalog>,
    /// The session's remembered agent choices.
    pub(crate) memory: &'a SelectionMemory,
    /// Whether the pick belongs to a draft or an open session.
    pub(crate) scope: SessionScope,
}

/// What the selection becomes, and whether the model must be remembered for the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentOutcome {
    /// The model the agent lands on.
    pub(crate) model: Option<ModelKey>,
    /// The effective Thinking level.
    pub(crate) variant: Option<EffortChoice>,
    /// The explicit Thinking level, when a saved choice or a live pick made one.
    pub(crate) variant_override: Option<EffortChoice>,
    /// Where the model came from.
    pub(crate) model_source: SelectionSource,
    /// The live pick or a kept manual pair is remembered for the agent.
    pub(crate) remember_model: bool,
}

/// Resolves the model and Thinking level picking `agent_name` lands on.
pub(crate) fn resolve_agent_selection(
    context: AgentContext<'_>,
    current: &Selection,
    agent_name: &str,
) -> AgentOutcome {
    let agent = context
        .catalog
        .and_then(|catalog| catalog.agent(agent_name));
    if let Some(outcome) = live_pick(current) {
        return outcome;
    }
    if let Some(key) = context.memory.agent_model(agent_name) {
        return resolved(
            context,
            agent,
            agent_name,
            key,
            current,
            SelectionSource::Manual,
            false,
        );
    }
    if let Some(key) = pinned_catalog_model(agent, context.catalog) {
        return resolved(
            context,
            agent,
            agent_name,
            &key,
            current,
            SelectionSource::Auto,
            false,
        );
    }
    if let Some(outcome) = manual_pair(current) {
        return outcome;
    }
    if let Some(key) = settings_default_model(context.settings, context.catalog) {
        return resolved(
            context,
            agent,
            agent_name,
            &key,
            current,
            SelectionSource::Auto,
            false,
        );
    }
    unchanged(current)
}

fn live_pick(current: &Selection) -> Option<AgentOutcome> {
    let ModelPick::Live(picked) = &current.model_pick else {
        return None;
    };
    (current.model.as_ref() == Some(picked))
        .then(|| keep(picked, current, SelectionSource::Manual, true))
}

fn manual_pair(current: &Selection) -> Option<AgentOutcome> {
    let key = current
        .model
        .as_ref()
        .filter(|_| current.model_source == SelectionSource::Manual)?;
    Some(keep(key, current, SelectionSource::Manual, true))
}

fn unchanged(current: &Selection) -> AgentOutcome {
    AgentOutcome {
        model: current.model.clone(),
        variant: current.variant.clone(),
        variant_override: current.variant_override.clone(),
        model_source: current.model_source,
        remember_model: false,
    }
}

fn resolved(
    context: AgentContext<'_>,
    agent: Option<&Agent>,
    agent_name: &str,
    key: &ModelKey,
    current: &Selection,
    source: SelectionSource,
    remember_model: bool,
) -> AgentOutcome {
    let (variant, variant_override) = resolved_variant(context, agent, agent_name, key, current);
    AgentOutcome {
        model: Some(key.clone()),
        variant,
        variant_override,
        model_source: source,
        remember_model,
    }
}

fn keep(
    key: &ModelKey,
    current: &Selection,
    source: SelectionSource,
    remember_model: bool,
) -> AgentOutcome {
    AgentOutcome {
        model: Some(key.clone()),
        variant: current.variant.clone(),
        variant_override: current.variant_override.clone(),
        model_source: source,
        remember_model,
    }
}

fn pinned_catalog_model(agent: Option<&Agent>, catalog: Option<&Catalog>) -> Option<ModelKey> {
    let pin = agent?.model.as_ref()?;
    if pin.provider_id.is_empty() || pin.id.is_empty() {
        return None;
    }
    let key = ModelKey::new(&pin.provider_id, &pin.id);
    catalog?.has_model(&key).then_some(key)
}

fn settings_default_model(settings: &Settings, catalog: Option<&Catalog>) -> Option<ModelKey> {
    let key = ModelKey::parse(settings.default_model.as_deref()?)?;
    catalog?.has_model(&key).then_some(key)
}

#[cfg(test)]
#[path = "set_agent_test.rs"]
mod set_agent_test;
