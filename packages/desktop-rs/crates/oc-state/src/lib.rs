//! Pure state reducer that turns server responses and events into view state.
//!
//! [`State`] holds what the window draws; [`State::apply`] takes an [`Action`]: a wire event
//! from `GET /api/event`, or what the app learned from its own calls (connection changes,
//! loaded or failed lists, prompt results, composer edits). The sidebar's own inputs (the
//! shared settings, the home info and the global session list) arrive through
//! [`State::apply_sidebar`], and [`State::sidebar_model`] derives the sidebar from them. No
//! I/O, no clock and no GPUI, so a fixture replay gives the same state every run.

mod action;
mod auto_accept;
mod control;
mod events;
#[cfg(test)]
mod fixture_test;
mod form;
mod form_drafts;
mod form_serializers;
mod ids;
mod interrupted;
mod load;
mod loaded;
mod logging;
mod message;
mod message_events;
mod permission_mode;
mod permission_patterns;
mod permission_summary;
mod recoverable;
mod requests;
mod retry;
mod safety_hold;
mod scope;
pub mod search;
mod selection;
mod session_entry;
mod session_list;
mod session_requests;
mod settings_overlay;
mod sidebar;
mod sidebar_state;
mod state;
mod status;
mod tool_events;
mod turn_actions;
mod websearch;

pub use action::Action;
pub use form::{
    ExternalLinks, FieldValue, FormEvaluation, FormValues, build_form_answer, evaluate_form,
    field_answer, initial_form_values, is_answerable_field, missing_required_keys, value_of,
    visible_fields,
};
pub use form_drafts::{FormDraft, FormDraftScope, FormDrafts, fields_signature};
pub use form_serializers::serialize_form_answer_as_markdown;
pub use ids::{CallId, MessageId, RequestId, SessionId};
pub use message::{
    AssistantMessage, ChatMessage, Delivery, Item, StepTiming, TextItem, ToolItem, UserMessage,
};
pub use oc_client::filesystem::Home;
pub use oc_client::types::{ModelRef, ToolStatus, WorkState};
pub use oc_client::{Project, ProjectPatch, Settings, SettingsPatch};
pub use permission_mode::{SafetyNet, displayed_permission_mode, next_permission_mode};
pub use permission_patterns::get_visible_permission_patterns;
pub use permission_summary::{
    PermissionSummary, PermissionSummaryKind, PermissionTarget, can_save_permission,
    describe_save_patterns, strip_directory_glob, summarize_permission,
};
pub use recoverable::{
    ClosedOutcome, ErrorOrigin, QuestionAskOutcome, QuestionToolLink, RecordedOutcome,
    RecoveredQuestion, RecoveryEvidence, classify_question_tool_error, question_tool_link,
};
pub use requests::ScopedPermission;
pub use safety_hold::SafetyHold;
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
pub use websearch::{
    WebSearchConsent, WebSearchConsentOption, WebSearchConsentStep, read_web_search_consent,
};
