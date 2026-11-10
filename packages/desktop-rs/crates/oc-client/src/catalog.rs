//! The catalog and OpenCode config the pickers read for one directory.

use serde::Deserialize;

use crate::types::ModelRef;

/// A provider as `GET /api/provider` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Provider {
    /// Provider id, as sessions and settings refer to it.
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Whether the user switched the provider off; every other value is offerable.
    #[serde(default)]
    pub activation: ProviderActivation,
}

/// How a provider is switched on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderActivation {
    /// The provider is on.
    Enabled,
    /// The user switched the provider off; its models are not offered.
    Disabled,
    /// A value this client does not know; treated as on.
    #[default]
    #[serde(other)]
    Other,
}

/// A model as `GET /api/model?types=chat&types=image` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Model {
    /// Catalog id: what sessions, settings refs and comparisons use.
    pub id: String,
    /// Provider id.
    #[serde(rename = "providerID", default)]
    pub provider_id: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Thinking levels the model offers; empty when it has none.
    #[serde(default)]
    pub variants: Vec<ModelVariant>,
    /// Whether the model is offered; a model missing this is not.
    #[serde(default)]
    pub enabled: bool,
    /// Chat, image, or something else; an unset type is chat.
    #[serde(rename = "type", default)]
    pub kind: ModelKind,
}

/// One Thinking level of a model.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModelVariant {
    /// Variant id, sent as `variant`.
    pub id: String,
}

/// What a catalog model produces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    /// Text chat; an unset type means this.
    #[default]
    Chat,
    /// Image generation.
    Image,
    /// A kind this client does not offer.
    #[serde(other)]
    Other,
}

/// An agent as `GET /api/agent` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Agent {
    /// Agent name, sent as `agent`.
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// What the agent may be picked for; an unset mode counts as `all`.
    #[serde(default)]
    pub mode: AgentMode,
    /// Whether the user hid the agent from the picker.
    #[serde(default)]
    pub hidden: bool,
    /// The model the agent pins, if it pins one.
    #[serde(default)]
    pub model: Option<ModelRef>,
}

/// What an agent is offered for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// The main agent a session starts on.
    Primary,
    /// Pickable in both positions; an unset mode means this.
    #[default]
    All,
    /// Only called by another agent; never offered.
    Subagent,
    /// A mode this client does not know; never offered.
    #[serde(other)]
    Other,
}

/// One source of `GET /api/config`; only `document` sources carry settings.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConfigSource {
    /// The document this source adds; absent for a directory source.
    #[serde(default)]
    pub info: Option<serde_json::Value>,
}

/// The OpenCode config fields this app reads, folded from the sources in order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct OpenCodeConfig {
    /// The model OpenCode uses when neither the agent nor the settings pick one.
    #[serde(default)]
    pub model: Option<ConfigModel>,
    /// The agent OpenCode starts a session on when no other default is set.
    #[serde(rename = "default_agent", default)]
    pub default_agent: Option<String>,
}

/// A model reference in the OpenCode config: `providerID` plus `model` (or `modelID`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConfigModel {
    /// Provider id.
    #[serde(rename = "providerID")]
    pub provider_id: String,
    /// Model id, under either key OpenCode has used.
    #[serde(alias = "modelID")]
    pub model: String,
    /// Variant the config pins with the model.
    #[serde(default)]
    pub variant: Option<String>,
}

#[cfg(test)]
#[path = "catalog_test.rs"]
mod catalog_test;
