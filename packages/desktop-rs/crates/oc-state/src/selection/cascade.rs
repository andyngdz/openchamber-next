//! The default agent and model cascade (`useConfigStore.ts:436-581`).
//!
//! Agent: project `defaultAgent`, settings `defaultAgent`, the OpenCode config's
//! `default_agent` when it names a visible primary agent, `build`, the first primary agent,
//! the first agent. Model: project `defaultModel`, settings `defaultModel`, the agent's
//! pinned model, the OpenCode config `model`, `lastSelectedModel`, `opencode/big-pickle`,
//! the first model not hidden, `opencode/big-pickle` even if hidden, the first model.
//!
//! A configured default, an agent pin and `lastSelectedModel` stay selected when the catalog
//! lacks their model, and their variant is then left unchecked. The variant falls back to
//! the agent's pin for that model, then `lastUsedEfforts`, and only a level the model offers
//! is kept when the catalog has it.

use oc_client::catalog::{Agent, Model, ModelVariant};
use oc_client::settings::Settings;

use super::catalog::{Catalog, agent_is_primary};
use super::value::{EffortChoice, ModelKey, Selection};

/// The default agent, model and variant a project stores.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProjectDefaults<'a> {
    /// `defaultAgent`.
    pub(crate) agent: Option<&'a str>,
    /// `defaultModel`, as `provider/id`.
    pub(crate) model: Option<&'a str>,
    /// `defaultVariant`, which belongs to the project's model.
    pub(crate) variant: Option<&'a str>,
}

/// Resolves the selection a fresh draft starts on.
pub(crate) fn resolve_default_selection(
    settings: &Settings,
    project: ProjectDefaults<'_>,
    catalog: Option<&Catalog>,
) -> Selection {
    let agent = resolve_agent(settings, project, catalog);
    let (model, variant) = resolve_model(settings, project, catalog, agent);
    let mut selection = Selection::auto(agent.map(|agent| agent.id.clone()), model);
    selection.variant = match variant {
        Some(choice) => Some(choice),
        None => agent_or_remembered(settings, catalog, agent, selection.model.as_ref()),
    };
    selection
}

fn resolve_agent<'a>(
    settings: &Settings,
    project: ProjectDefaults<'_>,
    catalog: Option<&'a Catalog>,
) -> Option<&'a Agent> {
    let catalog = catalog?;
    let agents = catalog.agents();
    if let Some(agent) = project.agent.and_then(|name| catalog.agent(name)) {
        return Some(agent);
    }
    if let Some(agent) = settings
        .default_agent
        .as_deref()
        .and_then(|name| catalog.agent(name))
    {
        return Some(agent);
    }
    if let Some(candidate) = catalog
        .config()
        .default_agent
        .as_deref()
        .and_then(|name| catalog.agent(name))
        && agent_is_primary(candidate)
        && !candidate.hidden
    {
        return Some(candidate);
    }
    let primary: Vec<&Agent> = agents
        .iter()
        .filter(|agent| agent_is_primary(agent) && !agent.hidden)
        .collect();
    primary
        .iter()
        .find(|agent| agent.id == "build")
        .copied()
        .or_else(|| primary.first().copied())
        .or_else(|| agents.first())
}

fn resolve_model(
    settings: &Settings,
    project: ProjectDefaults<'_>,
    catalog: Option<&Catalog>,
    agent: Option<&Agent>,
) -> (Option<ModelKey>, Option<EffortChoice>) {
    if let Some(key) = project
        .model
        .or(settings.default_model.as_deref())
        .and_then(ModelKey::parse)
    {
        let configured = if project.model.is_some() {
            project.variant
        } else {
            settings.default_variant.as_deref()
        };
        let variant = checked_variant(catalog, &key, configured);
        return (Some(key), variant);
    }
    if let Some((key, variant)) = agent.and_then(pinned_model) {
        let variant = checked_variant(catalog, &key, variant);
        return (Some(key), variant);
    }
    if let Some((key, variant)) = config_model(catalog) {
        let variant = checked_variant(catalog, &key, variant);
        return (Some(key), variant);
    }
    (fallback_model(settings, catalog), None)
}

fn pinned_model(agent: &Agent) -> Option<(ModelKey, Option<&str>)> {
    let model = agent.model.as_ref()?;
    if model.provider_id.is_empty() || model.id.is_empty() {
        return None;
    }
    let key = ModelKey::new(&model.provider_id, &model.id);
    Some((key, model.variant.as_deref()))
}

fn config_model(catalog: Option<&Catalog>) -> Option<(ModelKey, Option<&str>)> {
    let model = catalog?.config().model.as_ref()?;
    let key = ModelKey::new(&model.provider_id, &model.model);
    Some((key, model.variant.as_deref()))
}

fn fallback_model(settings: &Settings, catalog: Option<&Catalog>) -> Option<ModelKey> {
    if let Some(key) = settings
        .last_selected_model
        .as_deref()
        .and_then(ModelKey::parse)
        && !is_model_hidden(settings, &key)
    {
        return Some(key);
    }
    first_visible_model(settings, catalog?)
}

fn first_visible_model(settings: &Settings, catalog: &Catalog) -> Option<ModelKey> {
    let big_pickle = ModelKey::new("opencode", "big-pickle");
    let big_pickle_visible =
        catalog.has_model(&big_pickle) && !is_model_hidden(settings, &big_pickle);
    if big_pickle_visible {
        return Some(big_pickle);
    }
    for provider in catalog.providers() {
        let visible = provider.models.iter().find(|model| {
            !is_model_hidden(settings, &ModelKey::new(&provider.provider.id, &model.id))
        });
        if let Some(model) = visible {
            return Some(ModelKey::new(&provider.provider.id, &model.id));
        }
    }
    if catalog.has_model(&big_pickle) {
        return Some(big_pickle);
    }
    catalog.providers().first().and_then(|provider| {
        provider
            .models
            .first()
            .map(|model| ModelKey::new(&provider.provider.id, &model.id))
    })
}

fn agent_or_remembered(
    settings: &Settings,
    catalog: Option<&Catalog>,
    agent: Option<&Agent>,
    key: Option<&ModelKey>,
) -> Option<EffortChoice> {
    let key = key?;
    let model = catalog?.model(key)?;
    if model.variants.is_empty() {
        return None;
    }
    let agent_variant = agent
        .and_then(|agent| agent.model.as_ref())
        .filter(|pin| pin.provider_id == key.provider_id() && pin.id == key.id())
        .and_then(|pin| pin.variant.as_deref())
        .map(|name| EffortChoice::Named(name.to_owned()));
    resolve_effort_choice(
        &[agent_variant, remembered_effort(settings, key)],
        &model.variants,
    )
}

/// A configured variant, checked only when the catalog has its model.
pub(crate) fn checked_variant(
    catalog: Option<&Catalog>,
    key: &ModelKey,
    variant: Option<&str>,
) -> Option<EffortChoice> {
    let variant = variant.filter(|value| !value.is_empty())?;
    if let Some(model) = catalog.and_then(|catalog| catalog.model(key))
        && !offers_variant(model, variant)
    {
        return None;
    }
    Some(EffortChoice::Named(variant.to_owned()))
}

/// The first choice a source names: an explicit Default stops the search, a level the model
/// does not offer is skipped (`resolveEffortChoice`, `modelEfforts.ts:49-57`).
pub(crate) fn resolve_effort_choice(
    sources: &[Option<EffortChoice>],
    variants: &[ModelVariant],
) -> Option<EffortChoice> {
    for source in sources.iter().flatten() {
        match source {
            EffortChoice::Default => return Some(EffortChoice::Default),
            EffortChoice::Named(name) if variants.iter().any(|variant| variant.id == *name) => {
                return Some(EffortChoice::Named(name.clone()));
            }
            EffortChoice::Named(_) => {}
        }
    }
    None
}

/// The level this model was last sent with, across sessions and projects.
pub(crate) fn remembered_effort(settings: &Settings, key: &ModelKey) -> Option<EffortChoice> {
    EffortChoice::from_settings(settings.last_used_efforts.get(&key.settings_key()))
}

/// Whether the model offers the named Thinking level.
pub(crate) fn offers_variant(model: &Model, variant: &str) -> bool {
    model.variants.iter().any(|entry| entry.id == variant)
}

/// Whether the user hid the model from the picker.
pub(crate) fn is_model_hidden(settings: &Settings, key: &ModelKey) -> bool {
    settings
        .hidden_models
        .iter()
        .any(|hidden| hidden.provider_id == key.provider_id() && hidden.model_id == key.id())
}

#[cfg(test)]
#[path = "cascade_test.rs"]
mod cascade_test;
