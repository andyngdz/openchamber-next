//! OpenChamber desktop spike: the GPUI window.
//!
//! The window draws `oc-state`'s state in the layout of the React desktop app, fed by the
//! OpenChamber server it runs as a sidecar. With
//! `--dump-layout` it builds that state from a fixture file instead, renders it in a headless
//! window, and writes where every region and control landed, for the parity test against
//! `fixtures/reference/`; with `--screenshot` it renders the same headless frame as a PNG.

mod app;
mod errors;
mod fixture;
mod icons;
mod layout;
mod live;
mod logging;
mod root;
mod server;
mod theme;
mod views;

pub use app::{Launch, log_failure, run};
pub use errors::AppError;
pub use live::chat_folder_for;
