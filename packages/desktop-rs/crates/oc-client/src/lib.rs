//! HTTP and event-stream client for the OpenChamber server.
//!
//! The server answers on `http://127.0.0.1:<port>`; [`Client`] covers the calls the
//! desktop app needs and decodes them into the hand-written types in [`types`], [`catalog`],
//! [`settings`], [`event`], [`control`], [`permission`], [`form`], [`queue`] and
//! [`routing`]. One client serves every project: a scoped call takes a [`Directory`], and
//! [`Client::events`] follows the one global stream.

mod calls;
pub mod catalog;
mod client;
pub mod control;
mod directory;
mod error;
pub mod event;
pub mod filesystem;
pub mod follow_up;
pub mod form;
pub mod form_field;
mod message_id;
mod metadata;
mod payload;
pub mod permission;
pub mod project;
mod prompt;
pub mod queue;
mod queue_calls;
mod requests;
pub mod routing;
pub mod session_page;
pub mod settings;
pub mod settings_patch;
mod sort_order;
mod sse;
mod tool_payload;
mod tool_state;
mod turn_calls;
pub mod types;
mod uri_component;

pub use client::{Client, ControlStream, EventStream};
pub use directory::Directory;
pub use error::ClientError;
pub use event::{Event, EventData};
pub use follow_up::FollowUpBehavior;
pub use message_id::new_message_id;
pub use project::{Project, ProjectPatch};
pub use settings::{
    LastUsedEfforts, Settings, SettingsModelRef, SidebarSection, SidebarSectionKind,
    SidebarSortOrder,
};
pub use settings_patch::SettingsPatch;
