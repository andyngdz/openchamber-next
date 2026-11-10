//! The layout registry: where every region and control landed, and its JSON dump.

mod control_spec;
mod dump;
mod element;
mod registry;

pub(crate) use dump::LayoutDump;
pub(crate) use element::{content, control, region};
pub(crate) use registry::{Area, ControlSpec, LayoutRegistry, Reveal, Role};
