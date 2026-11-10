//! The Thinking level an agent switch lands on (`resolveVariantSelectionForModel`,
//! `useConfigStore.ts:3546-3590`).
//!
//! The session-agent memory wins, then a draft's live override for the same model, then the
//! inherited level: the agent's pin for the model, the configured default, the remembered
//! effort. A level the model does not offer is skipped; an explicit Default is a choice.

use oc_client::catalog::{Agent, Model};

use super::cascade::{offers_variant, remembered_effort, resolve_effort_choice};
use super::set_agent::{AgentContext, SessionScope};
use super::value::{EffortChoice, ModelKey, Selection};

/// The effective level and the explicit one, if a saved choice or live pick made one.
pub(super) fn resolved_variant(
    context: AgentContext<'_>,
    agent: Option<&Agent>,
    agent_name: &str,
    key: &ModelKey,
    current: &Selection,
) -> (Option<EffortChoice>, Option<EffortChoice>) {
    let model = context.catalog.and_then(|catalog| catalog.model(key));
    if model.is_some_and(|model| model.variants.is_empty()) {
        return (None, None);
    }
    let inherited = inherited_variant(context, agent, key);
    let saved = context.memory.agent_variant(agent_name, key).cloned();
    let live = if context.scope == SessionScope::Draft && current.model.as_ref() == Some(key) {
        current.variant_override.clone()
    } else {
        None
    };
    let explicit = saved
        .filter(|choice| choice_is_available(model, choice))
        .or_else(|| live.filter(|choice| choice_is_available(model, choice)));
    let effective = explicit.clone().or(inherited);
    (effective, explicit)
}

/// The agent's pin for this model, the configured default variant, then the remembered
/// effort (`resolveEffortChoice`).
pub(super) fn inherited_variant(
    context: AgentContext<'_>,
    agent: Option<&Agent>,
    key: &ModelKey,
) -> Option<EffortChoice> {
    let agent_variant = agent_pin_variant(agent, key);
    let configured = configured_variant(context.settings, context.project, key);
    match context.catalog.and_then(|catalog| catalog.model(key)) {
        Some(model) => resolve_effort_choice(
            &[
                agent_variant,
                configured,
                remembered_effort(context.settings, key),
            ],
            &model.variants,
        ),
        None => [agent_variant, configured].into_iter().flatten().next(),
    }
}

fn agent_pin_variant(agent: Option<&Agent>, key: &ModelKey) -> Option<EffortChoice> {
    let pin = agent?.model.as_ref()?;
    if pin.provider_id != key.provider_id() || pin.id != key.id() {
        return None;
    }
    pin.variant
        .as_deref()
        .filter(|variant| !variant.is_empty())
        .map(|variant| EffortChoice::Named(variant.to_owned()))
}

/// `getDefaultVariantForModel`: the project's variant only for its own model, then the
/// settings variant only for the settings model.
fn configured_variant(
    settings: &oc_client::settings::Settings,
    project: super::cascade::ProjectDefaults<'_>,
    key: &ModelKey,
) -> Option<EffortChoice> {
    let is_model = |value: Option<&str>| value.and_then(ModelKey::parse).as_ref() == Some(key);
    let variant = if is_model(project.model) {
        project.variant
    } else if is_model(settings.default_model.as_deref()) {
        settings.default_variant.as_deref()
    } else {
        None
    }?;
    (!variant.is_empty()).then(|| EffortChoice::Named(variant.to_owned()))
}

fn choice_is_available(model: Option<&Model>, choice: &EffortChoice) -> bool {
    match choice {
        EffortChoice::Default => true,
        EffortChoice::Named(variant) => model.is_none_or(|model| offers_variant(model, variant)),
    }
}

#[cfg(test)]
#[path = "variant_test.rs"]
mod variant_test;
