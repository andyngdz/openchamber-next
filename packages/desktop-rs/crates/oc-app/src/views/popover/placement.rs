//! Where each picker popover opens and how wide it is.

use gpui_kit::component::popover::Popover;
use gpui_kit::{Anchor, Styled, px};

use super::{PopoverKind, agent, model, project};

/// The popover's placement and surface for its kind.
pub(super) fn placed(popover: Popover, kind: PopoverKind) -> Popover {
    match kind {
        PopoverKind::Project => popover
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .offset(px(4.))
            .p(px(0.))
            .w(px(project::WIDTH)),
        PopoverKind::Model => popover
            .appearance(false)
            .anchor(Anchor::BottomRight)
            .offset(px(4.))
            .p(px(0.))
            .w(px(model::WIDTH))
            .max_h(px(model::MAX_HEIGHT)),
        PopoverKind::Agent => popover
            .appearance(false)
            .anchor(Anchor::BottomRight)
            .offset(px(4.))
            .p(px(0.))
            .w(px(agent::WIDTH)),
        PopoverKind::Thinking => popover
            .appearance(false)
            .anchor(Anchor::BottomRight)
            .offset(px(4.)),
    }
}
