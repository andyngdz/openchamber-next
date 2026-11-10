//! The catalog of one directory, built as React builds it.
//!
//! Providers the user did not switch off that have at least one enabled chat model are
//! offered, with their chat models in the order the server returned them
//! (`useConfigStore.ts:2405-2442`). Agents are kept as the server sent them; the picker's
//! own list filters and sorts them ([`Catalog::picker_agents`]).

use std::collections::HashMap;

use oc_client::catalog::{Agent, AgentMode, Model, ModelKind, OpenCodeConfig, Provider};

use super::value::ModelKey;

/// One provider and its offerable chat models, in the server's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModels {
    /// The provider as the server reports it.
    pub provider: Provider,
    /// Its enabled chat models, in the server's order.
    pub models: Vec<Model>,
}

/// The provider and agent catalog of one directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    providers: Vec<ProviderModels>,
    agents: Vec<Agent>,
    config: OpenCodeConfig,
}

impl Catalog {
    /// Groups `models` under `providers` as `useConfigStore.ts:2405-2442` does.
    ///
    /// A provider is kept when its activation is not `disabled` and it has at least one
    /// enabled chat model; every other provider (including an image-only one) is not
    /// offered. Models keep the server's order inside their provider.
    pub fn from_parts(
        providers: Vec<Provider>,
        models: Vec<Model>,
        agents: Vec<Agent>,
        config: OpenCodeConfig,
    ) -> Self {
        let mut by_provider: HashMap<String, Vec<Model>> = HashMap::new();
        for model in models.into_iter().filter(is_offerable_model) {
            by_provider
                .entry(model.provider_id.clone())
                .or_default()
                .push(model);
        }
        let providers = providers
            .into_iter()
            .filter(is_offerable_provider)
            .filter_map(|provider| {
                let models = by_provider.remove(&provider.id).unwrap_or_default();
                (!models.is_empty()).then_some(ProviderModels { provider, models })
            })
            .collect();
        Self {
            providers,
            agents,
            config,
        }
    }

    /// The OpenCode config fields the cascade reads.
    pub fn config(&self) -> &OpenCodeConfig {
        &self.config
    }

    /// The offerable providers, in the server's order.
    pub fn providers(&self) -> &[ProviderModels] {
        &self.providers
    }

    /// Every agent the server reported, in its order.
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// The agent picker's list: primary-mode, not hidden, sorted by lowercased name.
    ///
    /// React maps the server's `name` to `id` and `displayName` to `name`, so the sort
    /// key and the picker's name are this type's `id`.
    pub fn picker_agents(&self) -> Vec<&Agent> {
        let mut agents: Vec<&Agent> = self
            .agents
            .iter()
            .filter(|agent| agent_is_primary(agent) && !agent.hidden)
            .collect();
        agents.sort_by(|left, right| {
            left.id
                .to_lowercase()
                .cmp(&right.id.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        agents
    }

    /// The provider with `provider_id`, if the catalog offers it.
    pub fn provider(&self, provider_id: &str) -> Option<&ProviderModels> {
        self.providers
            .iter()
            .find(|provider| provider.provider.id == provider_id)
    }

    /// The model a key names, if the catalog offers it.
    pub fn model(&self, key: &ModelKey) -> Option<&Model> {
        self.provider(key.provider_id())
            .and_then(|provider| provider.models.iter().find(|model| model.id == key.id()))
    }

    /// Whether the catalog offers the model a key names.
    pub fn has_model(&self, key: &ModelKey) -> bool {
        self.model(key).is_some()
    }

    /// The agent the server reported under `id`.
    pub fn agent(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.id == id)
    }
}

fn is_offerable_model(model: &Model) -> bool {
    model.enabled && model.kind != ModelKind::Image
}

fn is_offerable_provider(provider: &Provider) -> bool {
    provider.activation != oc_client::catalog::ProviderActivation::Disabled
}

/// `isPrimaryMode`: `primary`, `all`, or a missing mode (`mobileControlsUtils.ts:9`).
pub(crate) fn agent_is_primary(agent: &Agent) -> bool {
    matches!(agent.mode, AgentMode::Primary | AgentMode::All)
}

#[cfg(test)]
#[path = "catalog_test.rs"]
mod catalog_test;
