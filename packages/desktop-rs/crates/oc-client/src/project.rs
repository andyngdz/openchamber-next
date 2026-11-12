//! One entry of the settings `projects` array, kept as the raw JSON object.
//!
//! React writes the array as a whole, so a client that only knows the fields it changes
//! must not drop the rest: `color`, `icon`, `iconBackground` and any field a newer React
//! adds survive a read-patch-write round trip.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The stored keys this app reads or patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Id,
    Path,
    Label,
    DefaultAgent,
    DefaultModel,
    DefaultVariant,
    Icon,
    Color,
    AddedAt,
    LastOpenedAt,
    SidebarCollapsed,
}

impl Field {
    fn key(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Path => "path",
            Self::Label => "label",
            Self::DefaultAgent => "defaultAgent",
            Self::DefaultModel => "defaultModel",
            Self::DefaultVariant => "defaultVariant",
            Self::Icon => "icon",
            Self::Color => "color",
            Self::AddedAt => "addedAt",
            Self::LastOpenedAt => "lastOpenedAt",
            Self::SidebarCollapsed => "sidebarCollapsed",
        }
    }
}

/// A project as the shared settings store it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Project(Map<String, Value>);

/// The project fields this app changes. Every other field of the stored object stays.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectPatch {
    /// The label the sidebar shows.
    pub label: Option<String>,
    /// The default agent of a new session in this project.
    pub default_agent: Option<String>,
    /// The default model (`provider/model`) of a new session in this project.
    pub default_model: Option<String>,
    /// The variant that belongs to the default model.
    pub default_variant: Option<String>,
    /// When the user last opened the project, in milliseconds since the epoch.
    pub last_opened_at: Option<u64>,
    /// Whether the project's sessions are collapsed in the sidebar.
    pub sidebar_collapsed: Option<bool>,
}

impl Project {
    /// The project id React builds from the path.
    pub fn id(&self) -> Option<&str> {
        self.text(Field::Id)
    }

    /// The absolute project directory.
    pub fn path(&self) -> Option<&str> {
        self.text(Field::Path)
    }

    /// The label the sidebar shows; React falls back to the folder name when this is absent.
    pub fn label(&self) -> Option<&str> {
        self.text(Field::Label)
    }

    /// The default agent of a new session in this project.
    pub fn default_agent(&self) -> Option<&str> {
        self.text(Field::DefaultAgent)
    }

    /// The default model of a new session in this project, as `provider/model`.
    pub fn default_model(&self) -> Option<&str> {
        self.text(Field::DefaultModel)
    }

    /// The variant that belongs to the default model.
    pub fn default_variant(&self) -> Option<&str> {
        self.text(Field::DefaultVariant)
    }

    /// The stored project icon key (`PROJECT_ICONS` in `lib/projectMeta.ts`).
    pub fn icon(&self) -> Option<&str> {
        self.text(Field::Icon)
    }

    /// The stored project color, a hex color.
    pub fn color(&self) -> Option<&str> {
        self.text(Field::Color)
    }

    /// When the project was added, in milliseconds since the epoch.
    pub fn added_at(&self) -> Option<u64> {
        self.number(Field::AddedAt)
    }

    /// When the user last opened the project, in milliseconds since the epoch.
    pub fn last_opened_at(&self) -> Option<u64> {
        self.number(Field::LastOpenedAt)
    }

    /// Whether the project's sessions are collapsed in the sidebar.
    pub fn sidebar_collapsed(&self) -> Option<bool> {
        self.0
            .get(Field::SidebarCollapsed.key())
            .and_then(Value::as_bool)
    }

    /// A copy of the project with every field the patch names replaced.
    pub fn with_patch(&self, patch: &ProjectPatch) -> Self {
        let mut fields = self.0.clone();
        set_text(&mut fields, Field::Label, patch.label.as_deref());
        set_text(
            &mut fields,
            Field::DefaultAgent,
            patch.default_agent.as_deref(),
        );
        set_text(
            &mut fields,
            Field::DefaultModel,
            patch.default_model.as_deref(),
        );
        set_text(
            &mut fields,
            Field::DefaultVariant,
            patch.default_variant.as_deref(),
        );
        if let Some(at) = patch.last_opened_at {
            fields.insert(Field::LastOpenedAt.key().to_owned(), Value::from(at));
        }
        if let Some(collapsed) = patch.sidebar_collapsed {
            fields.insert(
                Field::SidebarCollapsed.key().to_owned(),
                Value::from(collapsed),
            );
        }
        Self(fields)
    }

    fn text(&self, field: Field) -> Option<&str> {
        self.0.get(field.key()).and_then(Value::as_str)
    }

    fn number(&self, field: Field) -> Option<u64> {
        self.0.get(field.key()).and_then(Value::as_u64)
    }
}

fn set_text(fields: &mut Map<String, Value>, field: Field, value: Option<&str>) {
    if let Some(value) = value {
        fields.insert(field.key().to_owned(), Value::from(value));
    }
}

#[cfg(test)]
#[path = "project_test.rs"]
mod project_test;
