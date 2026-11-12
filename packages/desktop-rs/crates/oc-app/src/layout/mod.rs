//! The layout registry: where every region and control landed, and its JSON dump.

mod component;
mod control_spec;
pub(crate) mod draw;
mod dump;
mod element;
mod registry;

pub(crate) use component::ComponentStyle;
pub(crate) use dump::LayoutDump;
pub(crate) use element::{content, control, mark, placed, region};
pub(crate) use registry::{Area, ControlSpec, DrawSlot, LayoutRegistry, Reveal, Role};
