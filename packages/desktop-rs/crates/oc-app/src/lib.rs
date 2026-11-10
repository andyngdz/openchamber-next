//! OpenChamber desktop spike: the GPUI window.
//!
//! The window draws `oc-state`'s state in the layout of the React desktop app. With
//! `--dump-layout` it builds that state from a fixture file instead, renders it, and writes
//! where every region and control landed, for the parity test against `fixtures/reference/`.

mod app;
mod errors;
mod fixture;
mod layout;
mod logging;
mod root;
mod theme;
mod views;
mod work;

pub use app::{Launch, run};
pub use errors::AppError;
