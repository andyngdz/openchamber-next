//! Pure state reducer that turns server responses and events into view state.
//!
//! [`State`] holds what the window draws; [`State::apply`] takes an [`Action`]: a wire event
//! from `GET /api/event`, or what the app learned from its own calls (connection changes,
//! loaded or failed lists, prompt results, composer edits). The sidebar's own inputs (the
//! shared settings, the home info and the global session list) arrive through
//! [`State::apply_sidebar`], and [`State::sidebar_model`] derives the sidebar from them. No
//! I/O, no clock and no GPUI, so a fixture replay gives the same state every run.

mod action;
mod events;
#[cfg(test)]
mod fixture_test;
mod ids;
mod load;
mod loaded;
mod logging;
mod message;
mod message_events;
pub mod search;
mod selection;
mod session_entry;
mod session_list;
mod settings_overlay;
mod sidebar;
mod sidebar_state;
mod state;
mod status;

pub use action::Action;
pub use ids::{CallId, MessageId, RequestId, SessionId};
pub use message::{
    AssistantMessage, ChatMessage, Delivery, Item, StepTiming, TextItem, ToolItem, UserMessage,
};
pub use oc_client::filesystem::Home;
pub use oc_client::types::{ModelRef, ToolStatus, WorkState};
pub use oc_client::{Project, ProjectPatch, Settings, SettingsPatch};
pub use selection::{
    Catalog, DraftState, DraftTarget, EffortChoice, EnterKeys, ModelEntry, ModelKey, ModelSections,
    ProviderModels, ProviderSection, Selection, SelectionAction, SelectionSource, SendSteps,
    is_same_model_ref, model_sections, ranked_agents, ranked_projects, resolve_send_selection,
    should_submit_enter, wire_model,
};
pub use session_entry::{PendingRequest, SessionEntry};
pub use session_list::SessionList;
pub use sidebar::{
    MoreKey, ProjectGroup, SidebarModel, SidebarProject, SidebarRow, SidebarSectionModel,
};
pub use sidebar_state::SidebarAction;
pub use state::{Composer, OpenSession, State};
pub use status::{
    ComposerStatus, ConnectionStatus, LoadStatus, Region, ServerExit, ServerFailure, SessionStatus,
};
