//! The picker popup's kind and the value one row picks.

/// Which picker is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PopoverKind {
    /// The empty state's Choose project popup.
    Project,
    /// The composer's model picker.
    Model,
    /// The composer's agent menu.
    Agent,
    /// The composer's Thinking menu.
    Thinking,
}

/// What a popup row picks when Enter or a click lands on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Pick {
    /// The managed Chat target.
    Chat,
    /// One project from settings.
    Project(String),
    /// One catalog model.
    Model {
        /// Provider id.
        provider_id: String,
        /// Catalog model id.
        model_id: String,
    },
    /// One agent.
    Agent(String),
    /// One Thinking level; `None` is Default.
    Variant(Option<String>),
}

#[cfg(test)]
#[path = "types_test.rs"]
mod types_test;
