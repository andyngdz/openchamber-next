//! One recorded icon or text run, as the registry keeps it until the dump reads it.

use gpui_kit::{Bounds, Pixels, SharedString};

/// One recorded icon or text run of a control.
#[derive(Debug)]
pub(super) struct Recorded<T> {
    pub(super) control: SharedString,
    pub(super) draw: T,
    /// Where it was painted, when the element reported it.
    pub(super) bounds: Option<Bounds<Pixels>>,
    /// False once a later render of the same control replaced it.
    pub(super) live: bool,
}

impl<T: Clone> Recorded<T> {
    pub(super) fn new(control: &SharedString, draw: T) -> Self {
        Self {
            control: control.clone(),
            draw,
            bounds: None,
            live: true,
        }
    }

    pub(super) fn of(&self, control: &str) -> Option<(T, Option<Bounds<Pixels>>)> {
        (self.live && self.control.as_ref() == control).then(|| (self.draw.clone(), self.bounds))
    }
}

/// Where one recorded icon or text run sits in the frame, so the element that paints it can
/// report its own bounds with [`LayoutRegistry::place`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawSlot {
    Icon(usize),
    Text(usize),
}
