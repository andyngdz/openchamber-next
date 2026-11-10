//! The selection half of the window state: catalogs per directory, the draft's target and
//! selection, each open session's selection and remembered agent choices.
//!
//! Inputs arrive through [`State::apply_selection`], separate from [`crate::Action`] so the
//! spike's single-directory callers keep their exhaustive matches. Time is passed in with
//! the project pick, never read here.

use std::collections::BTreeMap;

use oc_client::types::SessionRecord;

use super::catalog::Catalog;
use super::memory::SelectionMemory;
use super::value::{DraftTarget, Selection};
use crate::ids::SessionId;
use crate::sidebar_state::Loaded;
use crate::state::State;

/// A new draft: its target and the selection a picker edits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftState {
    /// Chat or one project.
    pub target: DraftTarget,
    /// The agent, model and Thinking level the draft would send with.
    pub selection: Selection,
}

/// The selection inputs and choices, kept beside the sidebar's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SelectionState {
    /// The draft's target and selection.
    pub(crate) draft: DraftState,
    /// Each open session's selection, once its record arrives.
    pub(crate) selections: BTreeMap<SessionId, Selection>,
    /// The session records the send comparison reads.
    pub(crate) records: BTreeMap<SessionId, SessionRecord>,
    /// Each session's remembered agent choices.
    pub(crate) memories: BTreeMap<SessionId, SelectionMemory>,
    /// Catalogs by directory; a failure keeps the previous value.
    pub(crate) catalogs: BTreeMap<String, Loaded<Catalog>>,
}

/// One change to the pickers or the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionAction {
    /// The directory's providers, models, agents and config answered together.
    CatalogLoaded {
        /// The directory the catalog belongs to.
        directory: String,
        /// The built catalog.
        catalog: Catalog,
    },
    /// A directory's catalog failed; its popover shows the error with Retry.
    CatalogFailed {
        /// The directory the catalog belongs to.
        directory: String,
        /// Error text shown in the popover.
        message: String,
    },
    /// `GET /api/session/:id` answered: the session's selection starts from its record.
    SessionRecordLoaded {
        /// The session the record belongs to.
        session: SessionId,
        /// The record's agent and model.
        record: SessionRecord,
    },
    /// The user picked the Chat row.
    PickChat,
    /// The user picked a project; `now` stamps its `lastOpenedAt`.
    PickProject {
        /// The stored project id.
        id: String,
        /// Milliseconds since the epoch, supplied by the app.
        now: u64,
    },
    /// The user picked an agent.
    PickAgent(String),
    /// The user picked a model.
    PickModel {
        /// Provider id.
        provider_id: String,
        /// Catalog model id.
        model_id: String,
    },
    /// The user picked a Thinking level; `None` is Default.
    PickVariant(Option<String>),
    /// A prompt was accepted: the sent effort is remembered and the pick is no longer live.
    SendSucceeded,
    /// The user starred or unstarred a model.
    ToggleFavoriteModel {
        /// Provider id.
        provider_id: String,
        /// Catalog model id.
        model_id: String,
    },
    /// The user starred or unstarred an agent.
    ToggleFavoriteAgent(String),
}

impl State {
    /// Applies one selection change. Pure: time arrives with the action.
    pub fn apply_selection(&mut self, action: SelectionAction) {
        match action {
            SelectionAction::CatalogLoaded { directory, catalog } => {
                self.catalog_loaded(&directory, catalog);
            }
            SelectionAction::CatalogFailed { directory, message } => {
                self.catalog_failed(&directory, message);
            }
            SelectionAction::SessionRecordLoaded { session, record } => {
                self.session_record_loaded(&session, record);
            }
            SelectionAction::PickChat => self.pick_chat(),
            SelectionAction::PickProject { id, now } => self.pick_project(&id, now),
            SelectionAction::PickAgent(name) => self.pick_agent(&name),
            SelectionAction::PickModel {
                provider_id,
                model_id,
            } => self.pick_model(&provider_id, &model_id),
            SelectionAction::PickVariant(variant) => self.pick_variant(variant.as_deref()),
            SelectionAction::SendSucceeded => self.send_succeeded(),
            SelectionAction::ToggleFavoriteModel {
                provider_id,
                model_id,
            } => self.toggle_favorite_model(&provider_id, &model_id),
            SelectionAction::ToggleFavoriteAgent(name) => self.toggle_favorite_agent(&name),
        }
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod state_test;
