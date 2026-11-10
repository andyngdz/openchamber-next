//! HTTP and event-stream client for the OpenChamber server.
//!
//! The server answers on `http://127.0.0.1:<port>`; [`Client`] covers the calls the
//! desktop spike needs and decodes them into the hand-written types in [`types`] and
//! [`event`].

mod client;
mod error;
pub mod event;
mod message_id;
mod metadata;
mod payload;
mod tool_payload;
pub mod types;
mod uri_component;

pub use client::{Client, EventStream};
pub use error::ClientError;
pub use event::{Event, EventData};
pub use message_id::new_message_id;
