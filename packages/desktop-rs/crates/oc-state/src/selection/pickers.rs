//! The lists the pickers draw: model sections, ranked agents and ranked projects.
//!
//! The model list keeps its sections and order and only filters
//! (`matches_rank_query`, `ModelPickerList.tsx:594-638`); favorites come first, then Recent
//! without the favorites, then one open section per provider ordered by `providerOrder`.
//! The agent and project lists rank their items (`rank_by_query`).

use std::collections::{BTreeSet, HashMap};

use oc_client::catalog::{Agent, Model};
use oc_client::project::Project;
use oc_client::settings::Settings;

use super::cascade::is_model_hidden;
use super::catalog::{Catalog, ProviderModels};
use super::value::ModelKey;
use crate::search::{RankByQueryOptions, matches_rank_query, rank_by_query};

/// One model row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    /// Provider id, half of the model's identity.
    pub provider_id: String,
    /// Catalog model id, half of the model's identity.
    pub model_id: String,
    /// Display name: the server's name, else the id.
    pub name: String,
    /// Display name of the provider, else its id.
    pub provider_name: String,
    /// Thinking levels the model offers, in the server's order.
    pub variants: Vec<String>,
}

/// One provider section of the model list; every section is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSection {
    /// Provider id.
    pub provider_id: String,
    /// Display name: the server's name, else the id.
    pub name: String,
    /// The provider's matching models, in catalog order.
    pub models: Vec<ModelEntry>,
}

/// The model picker's derived sections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelSections {
    /// Starred models, in the order settings store them.
    pub favorites: Vec<ModelEntry>,
    /// Recently picked models without the favorites.
    pub recent: Vec<ModelEntry>,
    /// One open section per provider with a matching model.
    pub providers: Vec<ProviderSection>,
}

/// Builds the model picker's sections for `query`.
pub fn model_sections(catalog: &Catalog, settings: &Settings, query: &str) -> ModelSections {
    let favorite_pairs = favorite_pairs(settings);
    let favorites = settings
        .favorite_models
        .iter()
        .filter_map(|reference| {
            let key = ModelKey::new(&reference.provider_id, &reference.model_id);
            visible_entry(catalog, settings, &key).filter(|entry| entry_matches(entry, query))
        })
        .collect();
    let recent = settings
        .recent_models
        .iter()
        .filter(|reference| {
            !favorite_pairs.contains(&(reference.provider_id.clone(), reference.model_id.clone()))
        })
        .filter_map(|reference| {
            let key = ModelKey::new(&reference.provider_id, &reference.model_id);
            visible_entry(catalog, settings, &key).filter(|entry| entry_matches(entry, query))
        })
        .collect();
    let providers = ordered_providers(catalog, &settings.provider_order)
        .into_iter()
        .filter_map(|provider| provider_section(provider, settings, query))
        .collect();
    ModelSections {
        favorites,
        recent,
        providers,
    }
}

/// The agent picker's list for `query`, ranked on name, display name and description
/// (`ModelControls.tsx:666`).
pub fn ranked_agents<'a>(catalog: &'a Catalog, query: &str) -> Vec<&'a Agent> {
    let sorted = catalog.picker_agents();
    rank_by_query(
        &sorted,
        query,
        |agent| {
            vec![
                agent.id.as_str(),
                agent.display_name.as_str(),
                agent.description.as_str(),
            ]
        },
        RankByQueryOptions::default(),
    )
    .into_iter()
    .copied()
    .collect()
}

/// The Choose project list for `query`, ranked on label and path
/// (`DraftTargetSelectors.tsx:243`).
pub fn ranked_projects<'a>(settings: &'a Settings, query: &str) -> Vec<&'a Project> {
    rank_by_query(
        &settings.projects,
        query,
        |project| project.label().into_iter().chain(project.path()).collect(),
        RankByQueryOptions::default(),
    )
}

fn favorite_pairs(settings: &Settings) -> BTreeSet<(String, String)> {
    settings
        .favorite_models
        .iter()
        .map(|reference| (reference.provider_id.clone(), reference.model_id.clone()))
        .collect()
}

fn visible_entry(catalog: &Catalog, settings: &Settings, key: &ModelKey) -> Option<ModelEntry> {
    if is_model_hidden(settings, key) {
        return None;
    }
    model_entry(catalog, key)
}

fn model_entry(catalog: &Catalog, key: &ModelKey) -> Option<ModelEntry> {
    let provider = catalog.provider(key.provider_id())?;
    let model = provider.models.iter().find(|model| model.id == key.id())?;
    Some(entry_for(
        &provider.provider.id,
        &provider.provider.name,
        model,
    ))
}

fn entry_for(provider_id: &str, provider_name: &str, model: &Model) -> ModelEntry {
    ModelEntry {
        provider_id: provider_id.to_owned(),
        model_id: model.id.clone(),
        name: display_name(&model.name, &model.id),
        provider_name: display_name(provider_name, provider_id),
        variants: model
            .variants
            .iter()
            .map(|variant| variant.id.clone())
            .collect(),
    }
}

fn display_name(name: &str, fallback: &str) -> String {
    if name.trim().is_empty() {
        fallback.to_owned()
    } else {
        name.to_owned()
    }
}

fn entry_matches(entry: &ModelEntry, query: &str) -> bool {
    matches_rank_query(&[&entry.name, &entry.model_id, &entry.provider_name], query)
}

fn ordered_providers<'a>(
    catalog: &'a Catalog,
    provider_order: &[String],
) -> Vec<&'a ProviderModels> {
    if provider_order.is_empty() {
        return catalog.providers().iter().collect();
    }
    let rank: HashMap<&str, usize> = provider_order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let mut ranked: Vec<(usize, &ProviderModels)> = catalog
        .providers()
        .iter()
        .filter_map(|provider| {
            rank.get(provider.provider.id.as_str())
                .map(|index| (*index, provider))
        })
        .collect();
    ranked.sort_by_key(|(index, _)| *index);
    let unranked = catalog
        .providers()
        .iter()
        .filter(|provider| !rank.contains_key(provider.provider.id.as_str()));
    ranked
        .into_iter()
        .map(|(_, provider)| provider)
        .chain(unranked)
        .collect()
}

fn provider_section(
    provider: &ProviderModels,
    settings: &Settings,
    query: &str,
) -> Option<ProviderSection> {
    let models: Vec<ModelEntry> = provider
        .models
        .iter()
        .filter(|model| {
            !is_model_hidden(settings, &ModelKey::new(&provider.provider.id, &model.id))
        })
        .map(|model| entry_for(&provider.provider.id, &provider.provider.name, model))
        .filter(|entry| entry_matches(entry, query))
        .collect();
    if models.is_empty() {
        return None;
    }
    Some(ProviderSection {
        provider_id: provider.provider.id.clone(),
        name: display_name(&provider.provider.name, &provider.provider.id),
        models,
    })
}

#[cfg(test)]
#[path = "pickers_test.rs"]
mod pickers_test;
