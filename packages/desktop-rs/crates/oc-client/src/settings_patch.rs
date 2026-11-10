//! The partial settings document a write sends. A key left `None` is not in the body, so
//! the server keeps the value another client wrote.

use serde::{Deserialize, Serialize};

use crate::project::Project;
use crate::settings::{LastUsedEfforts, SettingsModelRef, SidebarSection, SidebarSortOrder};

/// A partial `PUT /api/config/settings?surface=desktop` body.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    /// The active project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_project_id: Option<String>,
    /// Starred models, newest first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite_models: Option<Vec<SettingsModelRef>>,
    /// Recently picked models, most recent first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_models: Option<Vec<SettingsModelRef>>,
    /// Models the user hid from the picker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_models: Option<Vec<SettingsModelRef>>,
    /// Starred agents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite_agents: Option<Vec<String>>,
    /// Recently picked agents, most recent first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_agents: Option<Vec<String>>,
    /// Provider sections the user collapsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed_model_providers: Option<Vec<String>>,
    /// The user's provider order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_order: Option<Vec<String>>,
    /// `provider/model` to the variant last used, or `null` for Default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_efforts: Option<LastUsedEfforts>,
    /// The model last picked in a chat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_selected_model: Option<String>,
    /// The settings default model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// The variant that belongs to the settings default model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_variant: Option<String>,
    /// The settings default agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_agent: Option<String>,
    /// Whether Enter sends when the user configured the setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enter_to_send: Option<bool>,
    /// Whether the user ever chose the Enter behaviour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enter_to_send_configured: Option<bool>,
    /// How the sidebar orders projects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_project_sort_order: Option<SidebarSortOrder>,
    /// Whether In work sessions stay in their project group too.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_keep_work_in_place: Option<bool>,
    /// Whether the Chats section is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_show_chats_section: Option<bool>,
    /// Whether the In work section is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_show_work_section: Option<bool>,
    /// Whether the Pinned section is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_show_pinned_section: Option<bool>,
    /// The sections in the user's order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_sections: Option<Vec<SidebarSection>>,
    /// Section ids this surface hides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_hidden_sections: Option<Vec<String>>,
    /// Whether the In work section exists at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_work_enabled: Option<bool>,
    /// The whole projects array, each object carrying the fields this app did not change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<Project>>,
}

impl SettingsPatch {
    /// Whether the body would be empty.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
#[path = "settings_patch_test.rs"]
mod settings_patch_test;
