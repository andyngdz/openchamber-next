//! Values a draft or session carries into the pickers and the send sequence.

use oc_client::types::ModelRef;
use oc_client::types::SessionRecord;

/// The variant OpenCode records for a model switched without one.
pub(crate) const DEFAULT_VARIANT: &str = "default";

/// Identity of a catalog model: its provider and the catalog `id`.
///
/// Everything this app stores or compares uses [`ModelKey::id`]: session and settings
/// references, and the `provider/id` keys of `lastUsedEfforts` and `lastSelectedModel`
/// (design, Model identity).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModelKey {
    provider_id: String,
    id: String,
}

impl ModelKey {
    /// A model of `provider_id` with catalog id `id`.
    pub fn new(provider_id: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            id: id.into(),
        }
    }

    /// The provider the model belongs to.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// The catalog model id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The `provider/id` spelling settings and OpenCode config use.
    pub fn settings_key(&self) -> String {
        format!("{}/{}", self.provider_id, self.id)
    }

    /// The `provider/id` spelling as a session or request model reference.
    pub fn to_model_ref(&self, variant: Option<&str>) -> ModelRef {
        ModelRef {
            id: self.id.clone(),
            provider_id: self.provider_id.clone(),
            variant: variant.map(str::to_owned),
        }
    }

    /// Splits a `provider/id` setting; the first `/` divides them (`modelIdentifier.ts:6-20`).
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let (provider_id, id) = value.split_once('/')?;
        if provider_id.is_empty() || id.is_empty() {
            return None;
        }
        Some(Self::new(provider_id, id))
    }
}

/// A Thinking level: a named level, or an explicit Default that sends no variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortChoice {
    /// The user picked Default; no `variant` is sent.
    Default,
    /// A named level the model offers.
    Named(String),
}

impl EffortChoice {
    /// The wire variant, if the choice names one.
    pub fn variant(&self) -> Option<&str> {
        match self {
            Self::Default => None,
            Self::Named(name) => Some(name),
        }
    }

    /// Reads one `lastUsedEfforts` entry; a missing key is no choice.
    pub(crate) fn from_settings(value: Option<&Option<String>>) -> Option<Self> {
        match value {
            Some(None) => Some(Self::Default),
            Some(Some(name)) => Some(Self::Named(name.clone())),
            None => None,
        }
    }
}

/// Where a selection's agent or model came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SelectionSource {
    /// The default cascade picked it; a fresh catalog may pick again.
    #[default]
    Auto,
    /// The open session's own record supplied it.
    Session,
    /// The user picked it.
    Manual,
}

/// Whether the composer holds an unsent model pick for its session.
///
/// A pick outranks the session's record and every remembered model until it is sent; the
/// agent switch reads it (`ModelControls.tsx:3559-3573`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum ModelPick {
    /// No unsent pick; the session's record or the cascade applies.
    #[default]
    None,
    /// The user picked this model and has not sent with it yet.
    Live(ModelKey),
}

/// The agent, model and Thinking level a draft or session holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub(crate) agent: Option<String>,
    pub(crate) model: Option<ModelKey>,
    pub(crate) variant: Option<EffortChoice>,
    pub(crate) variant_override: Option<EffortChoice>,
    pub(crate) agent_source: SelectionSource,
    pub(crate) model_source: SelectionSource,
    pub(crate) model_pick: ModelPick,
}

impl Selection {
    /// The agent name, once resolved.
    pub fn agent(&self) -> Option<&str> {
        self.agent.as_deref()
    }

    /// The model, once resolved.
    pub fn model(&self) -> Option<&ModelKey> {
        self.model.as_ref()
    }

    /// The effective Thinking level; `None` sends no variant.
    pub fn variant(&self) -> Option<&EffortChoice> {
        self.variant.as_ref()
    }

    /// Where the agent came from.
    pub fn agent_source(&self) -> SelectionSource {
        self.agent_source
    }

    /// Where the model came from.
    pub fn model_source(&self) -> SelectionSource {
        self.model_source
    }

    /// The session record's own choice; a missing field falls back to the cascade.
    pub(crate) fn from_record(record: &SessionRecord) -> Self {
        let model = record
            .model
            .as_ref()
            .filter(|model| !model.provider_id.is_empty() && !model.id.is_empty())
            .map(|model| ModelKey::new(&model.provider_id, &model.id));
        let model_source = record_source(model.as_ref());
        Self {
            agent: record.agent.clone(),
            model,
            variant: record.model.as_ref().and_then(record_effort),
            variant_override: None,
            agent_source: record_source(record.agent.as_ref()),
            model_source,
            model_pick: ModelPick::None,
        }
    }

    /// A cascade result with no user or session choice.
    pub(crate) fn auto(agent: Option<String>, model: Option<ModelKey>) -> Self {
        Self {
            agent,
            model,
            ..Self::default()
        }
    }
}

fn record_source<T>(value: Option<&T>) -> SelectionSource {
    match value {
        Some(_) => SelectionSource::Session,
        None => SelectionSource::Auto,
    }
}

/// OpenCode records a model switched without a variant as `"default"`.
fn record_effort(model: &ModelRef) -> Option<EffortChoice> {
    match model.variant.as_deref() {
        None | Some(DEFAULT_VARIANT) => None,
        Some(name) => Some(EffortChoice::Named(name.to_owned())),
    }
}

/// What a draft's target is: the managed Chat folder, or one project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DraftTarget {
    /// The Chat target; `activeProjectId` is left alone.
    #[default]
    Chat,
    /// One project from settings.
    Project {
        /// The stored project id.
        id: String,
        /// The project's absolute directory.
        directory: String,
    },
}

impl DraftTarget {
    /// The project id, when the target is a project.
    pub fn project_id(&self) -> Option<&str> {
        match self {
            Self::Chat => None,
            Self::Project { id, .. } => Some(id),
        }
    }

    /// The project directory, when the target is a project.
    pub fn directory(&self) -> Option<&str> {
        match self {
            Self::Chat => None,
            Self::Project { directory, .. } => Some(directory),
        }
    }
}

#[cfg(test)]
#[path = "value_test.rs"]
mod value_test;
