//! Pure state reducer that turns server responses and events into view state.
//!
//! [`State`] holds what the window draws; [`State::apply`] takes an [`Action`]: a wire event
//! from `GET /api/event`, or what the app learned from its own calls (connection changes,
//! loaded or failed lists, prompt results, composer edits). No I/O, no clock and no GPUI, so
//! a fixture replay gives the same state every run.

mod action;
mod events;
#[cfg(test)]
mod fixture_test;
mod ids;
mod load;
mod logging;
mod message;
mod message_events;
mod session_list;
mod state;
mod status;

pub use action::Action;
pub use ids::{CallId, MessageId, RequestId, SessionId};
pub use message::{
    AssistantMessage, ChatMessage, Delivery, Item, StepTiming, TextItem, ToolItem, UserMessage,
};
pub use oc_client::types::{ModelRef, ToolStatus, WorkState};
pub use session_list::{PendingRequest, SessionEntry, SessionList};
pub use state::{Composer, OpenSession, State};
pub use status::{
    ComposerStatus, ConnectionStatus, LoadStatus, Region, ServerExit, ServerFailure, SessionStatus,
};
