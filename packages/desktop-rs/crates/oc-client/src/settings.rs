//! The settings keys this app reads, decoded from `GET /api/config/settings?surface=desktop`.
//!
//! The document is the shared registry React writes; keys this app does not act on are
//! ignored. Writes are partial [`SettingsPatch`] objects, never a whole document.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::project::Project;

/// How the sidebar orders projects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SidebarSortOrder {
    /// The order the projects array holds.
    #[default]
    Manual,
    /// Label or path, ascending.
    AZ,
    /// Label or path, descending.
    ZA,
    /// Newest `addedAt` first.
    DateAdded,
    /// Newest session activity first.
    Recent,
}

/// One section of the sidebar, in the stored order. Custom sections carry more fields;
/// this app only places the built-in ones and hides by id.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SidebarSection {
    /// `builtin` for the four sections every client has, `custom` for a user's own.
    pub kind: SidebarSectionKind,
    /// `chats`, `pinned`, `work`, `projects`, or a custom section id.
    pub id: String,
}

/// What a sidebar section entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarSectionKind {
    /// Chats, Pinned, In work or Projects.
    Builtin,
    /// A user-defined filter section.
    Custom,
    /// A kind this client does not know; it is left out of the rendered order.
    #[serde(other)]
    Other,
}

/// A model reference as the settings store it: `modelID` holds the catalog `id`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SettingsModelRef {
    /// Provider id.
    #[serde(rename = "providerID")]
    pub provider_id: String,
    /// Catalog model id.
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// The settings document, as far as this app reads it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// The project the last pick made active.
    #[serde(default)]
    pub active_project_id: Option<String>,
    /// Starred models, newest first.
    #[serde(default)]
    pub favorite_models: Vec<SettingsModelRef>,
    /// Recently picked models, most recent first.
    #[serde(default)]
    pub recent_models: Vec<SettingsModelRef>,
    /// Models the user hid from the picker.
    #[serde(default)]
    pub hidden_models: Vec<SettingsModelRef>,
    /// Starred agents.
    #[serde(default)]
    pub favorite_agents: Vec<String>,
    /// Recently picked agents, most recent first.
    #[serde(default)]
    pub recent_agents: Vec<String>,
    /// Provider sections the user collapsed.
    #[serde(default)]
    pub collapsed_model_providers: Vec<String>,
    /// The user's provider order; providers not named keep the server's order after them.
    #[serde(default)]
    pub provider_order: Vec<String>,
    /// `provider/model` to the variant last used, or `null` for Default.
    #[serde(default)]
    pub last_used_efforts: BTreeMap<String, Option<String>>,
    /// The model last picked in a chat, as `provider/model`.
    #[serde(default)]
    pub last_selected_model: Option<String>,
    /// The settings default model, as `provider/model`.
    #[serde(default)]
    pub default_model: Option<String>,
    /// The variant that belongs to the settings default model.
    #[serde(default)]
    pub default_variant: Option<String>,
    /// The settings default agent.
    #[serde(default)]
    pub default_agent: Option<String>,
    /// Whether Enter sends when the user configured the setting.
    #[serde(default)]
    pub enter_to_send: bool,
    /// Whether the user ever chose the Enter behaviour.
    #[serde(default)]
    pub enter_to_send_configured: bool,
    /// How the sidebar orders projects.
    #[serde(default)]
    pub sidebar_project_sort_order: SidebarSortOrder,
    /// Whether In work sessions stay in their project group too.
    #[serde(default)]
    pub sidebar_keep_work_in_place: bool,
    /// Whether the Chats section is shown.
    #[serde(default = "visible_section")]
    pub sidebar_show_chats_section: bool,
    /// Whether the In work section is shown.
    #[serde(default = "visible_section")]
    pub sidebar_show_work_section: bool,
    /// Whether the Pinned section is shown.
    #[serde(default = "visible_section")]
    pub sidebar_show_pinned_section: bool,
    /// The sections in the order the user arranged them.
    #[serde(default)]
    pub sidebar_sections: Vec<SidebarSection>,
    /// Section ids this surface hides.
    #[serde(default)]
    pub sidebar_hidden_sections: Vec<String>,
    /// Whether the In work section exists at all.
    #[serde(default = "visible_section")]
    pub session_work_enabled: bool,
    /// The projects, kept as raw objects so unknown fields survive a patch.
    #[serde(default)]
    pub projects: Vec<Project>,
}

fn visible_section() -> bool {
    true
}

#[cfg(test)]
#[path = "settings_test.rs"]
mod settings_test;
