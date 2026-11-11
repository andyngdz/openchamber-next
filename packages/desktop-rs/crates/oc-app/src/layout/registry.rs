//! Where each region and control landed in the last frame.
//!
//! GPUI does not expose final element bounds for walking, so every region and control records
//! its bounds while the frame is prepainted. The root view clears the registry when it renders,
//! and prepaint visits elements in tree order, so controls come out in the order a DOM walk of
//! the React app lists them.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_kit::{Bounds, Pixels, SharedString};

pub(crate) use super::control_spec::ControlSpec;
use serde::Serialize;

/// The six regions the parity reference measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Area {
    Sidebar,
    Header,
    ChatColumn,
    Composer,
    WorkStatusPanel,
    IconRail,
    /// The open picker popup; absent from a frame with no popup.
    Popover,
}

impl Area {
    /// The name the reference JSON uses.
    pub(crate) fn json_name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Header => "header",
            Self::ChatColumn => "chatColumn",
            Self::Composer => "composer",
            Self::WorkStatusPanel => "workStatusPanel",
            Self::IconRail => "iconRail",
            Self::Popover => "popover",
        }
    }
}

/// Accessible role of a control, as Chromium reports it in the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    Button,
    Separator,
    Textbox,
    Generic,
    Combobox,
    Option,
    Menuitem,
}

/// When a control is visible; hidden controls stay in the tree and keep their place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reveal {
    /// Always painted.
    Always,
    /// Painted while the pointer is over the element of this hover group.
    OnHover(SharedString),
    /// Laid out but transparent until the state shows it.
    Hidden,
}

/// A control and the bounds it was prepainted at.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedControl {
    pub(crate) spec: ControlSpec,
    pub(crate) bounds: Bounds<Pixels>,
}

#[derive(Debug, Default)]
struct Frame {
    content: Option<Bounds<Pixels>>,
    regions: BTreeMap<Area, Bounds<Pixels>>,
    controls: Vec<PlacedControl>,
}

/// Bounds of every region and control of the last frame, shared by the elements that fill it.
#[derive(Debug, Clone, Default)]
pub(crate) struct LayoutRegistry {
    frame: Rc<RefCell<Frame>>,
}

impl LayoutRegistry {
    /// Forgets the previous frame; the root view calls it when it renders.
    pub(crate) fn begin_frame(&self) {
        let mut frame = self.frame.borrow_mut();
        frame.content = None;
        frame.regions.clear();
        frame.controls.clear();
    }

    /// Records the bounds of the app's root element: the content area every other bound is
    /// measured from. The window frame GPUI Kit draws around it is not content.
    pub(crate) fn record_content(&self, bounds: Bounds<Pixels>) {
        let mut frame = self.frame.borrow_mut();
        frame.content = Some(bounds);
    }

    /// The content area, if the frame laid it out.
    pub(crate) fn content(&self) -> Option<Bounds<Pixels>> {
        self.frame.borrow().content
    }

    /// Records a region's bounds.
    pub(crate) fn record_region(&self, area: Area, bounds: Bounds<Pixels>) {
        self.frame.borrow_mut().regions.insert(area, bounds);
    }

    /// Appends a control; calls arrive in prepaint order.
    pub(crate) fn record_control(&self, spec: ControlSpec, bounds: Bounds<Pixels>) {
        self.frame
            .borrow_mut()
            .controls
            .push(PlacedControl { spec, bounds });
    }

    /// Bounds of a region, if it was laid out.
    pub(crate) fn region(&self, area: Area) -> Option<Bounds<Pixels>> {
        self.frame.borrow().regions.get(&area).copied()
    }

    /// The controls of a region in prepaint order.
    pub(crate) fn controls_in(&self, area: Area) -> Vec<PlacedControl> {
        self.frame
            .borrow()
            .controls
            .iter()
            .filter(|placed| placed.spec.area == area)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
#[path = "registry_test.rs"]
mod registry_test;
