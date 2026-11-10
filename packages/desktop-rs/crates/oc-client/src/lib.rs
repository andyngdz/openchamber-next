//! HTTP and event-stream client for the OpenChamber server.
//!
//! The server answers on `http://127.0.0.1:<port>`; [`Client`] covers the calls the
//! desktop app needs and decodes them into the hand-written types in [`types`], [`catalog`],
//! [`settings`] and [`event`]. One client serves every project: a scoped call takes a
//! [`Directory`], and [`Client::events`] follows the one global stream.

mod calls;
pub mod catalog;
mod client;
mod directory;
mod error;
pub mod event;
pub mod filesystem;
mod message_id;
mod metadata;
mod payload;
pub mod project;
mod requests;
pub mod session_page;
pub mod settings;
pub mod settings_patch;
mod sort_order;
mod tool_payload;
pub mod types;
mod uri_component;

pub use client::{Client, EventStream};
pub use directory::Directory;
pub use error::ClientError;
pub use event::{Event, EventData};
pub use message_id::new_message_id;
pub use project::{Project, ProjectPatch};
pub use settings::{
    LastUsedEfforts, Settings, SettingsModelRef, SidebarSection, SidebarSectionKind,
    SidebarSortOrder,
};
pub use settings_patch::SettingsPatch;
