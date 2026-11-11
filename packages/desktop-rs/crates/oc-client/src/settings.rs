//! The settings keys this app reads, decoded from `GET /api/config/settings?surface=desktop`.
//!
//! The document is the shared registry React writes; keys this app does not act on are
//! ignored. Writes are partial [`SettingsPatch`] objects, never a whole document.

use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::project::Project;

/// `provider/model` to the Thinking level last used, in the order the user last used them.
///
/// React keeps this key as an insertion-ordered JavaScript object and evicts the oldest past
/// 512 (`useUIStore.ts:3268-3286`), so it decodes and encodes in document order instead of a
/// sorted map. `None` is an explicit Default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LastUsedEfforts(Vec<(String, Option<String>)>);

impl LastUsedEfforts {
    /// The level recorded for `key`, when the model has one.
    pub fn get(&self, key: &str) -> Option<&Option<String>> {
        self.0
            .iter()
            .find(|(entry, _)| entry == key)
            .map(|(_, value)| value)
    }

    /// How many models have a recorded level.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no model has a recorded level.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Writes `key` as the newest entry, moving an existing one to the end.
    pub fn insert(&mut self, key: impl Into<String>, value: Option<String>) {
        let key = key.into();
        self.0.retain(|(entry, _)| *entry != key);
        self.0.push((key, value));
    }

    /// Removes `key` and returns its level, when it had one.
    pub fn remove(&mut self, key: &str) -> Option<Option<String>> {
        let index = self.0.iter().position(|(entry, _)| entry == key)?;
        let (_, value) = self.0.remove(index);
        Some(value)
    }

    /// Drops the oldest entries so at most `keep` remain.
    pub fn keep_newest(&mut self, keep: usize) {
        if self.0.len() > keep {
            self.0.drain(..self.0.len() - keep);
        }
    }

    /// The entries from oldest to newest, as the settings document stores them.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Option<String>)> {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }
}

impl<'de> Deserialize<'de> for LastUsedEfforts {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EffortsVisitor;

        impl<'de> Visitor<'de> for EffortsVisitor {
            type Value = LastUsedEfforts;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a map of provider/model to a Thinking level")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut efforts = LastUsedEfforts::default();
                while let Some((key, value)) = map.next_entry::<String, Option<String>>()? {
                    efforts.insert(key, value);
                }
                Ok(efforts)
            }
        }

        deserializer.deserialize_map(EffortsVisitor)
    }
}

impl Serialize for LastUsedEfforts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

pub use crate::sort_order::SidebarSortOrder;

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
    pub last_used_efforts: LastUsedEfforts,
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
    /// The composer's behaviour for a send during a running turn; read through
    /// [`Settings::follow_up_behavior`].
    #[serde(default, rename = "followUpBehavior")]
    pub follow_up_behavior: Option<String>,
    /// The retired on/off key the behaviour replaced; read only when the new key is absent.
    #[serde(default, rename = "queueModeEnabled")]
    pub queue_mode_enabled: Option<bool>,
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
