//! Selection state: the catalog of a directory, the draft's target, each session's
//! selection, the default cascades, the picker rules and the send decision.
//!
//! The source of truth is the React composer: `useConfigStore.ts` for the cascades and the
//! agent switch, `useUIStore.ts` and `ModelControls.tsx` for the picker writes,
//! `ModelPickerList.tsx` for the model sections, `session-selection.ts` for the send, and
//! `composer/keyboardPolicy.ts` for Enter.

mod access;
mod cascade;
mod catalog;
mod draft;
mod memory;
mod patches;
mod pickers;
mod picks;
mod send;
mod set_agent;
mod state;
mod value;
mod variant;

pub use catalog::{Catalog, ProviderModels};
pub use pickers::{
    ModelEntry, ModelSections, ProviderSection, model_sections, ranked_agents, ranked_projects,
};
pub use send::{
    EnterKeys, SendSteps, is_same_model_ref, resolve_send_selection, should_submit_enter,
};
pub use state::{DraftState, SelectionAction};
pub use value::{DraftTarget, EffortChoice, ModelKey, Selection, SelectionSource};

pub(crate) use state::SelectionState;
