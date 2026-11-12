//! The icon rail on the right edge: panel switches, all inert in the spike.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Div, ParentElement, Styled, div, px};

use super::icon_button;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control, region};

/// The rail's buttons in reference order, in groups split by a divider.
const GROUPS: [&[(&str, RemixIcon)]; 3] = [
    &[
        ("Context", RemixIcon::DonutChartFill),
        ("Git", RemixIcon::GitBranch),
        ("Changes", RemixIcon::LayoutColumn),
        ("Walkthrough", RemixIcon::Route),
    ],
    &[
        ("Files", RemixIcon::FileEdit),
        ("Terminal", RemixIcon::TerminalBox),
        ("Browser", RemixIcon::Global),
    ],
    &[
        ("Project knowledge", RemixIcon::BookMarked),
        ("Configure panels", RemixIcon::Equalizer2),
    ],
];

/// The 44 px rail.
pub(crate) fn icon_rail(registry: &LayoutRegistry, cx: &App) -> Div {
    let theme = cx.theme();
    let mut rail = div()
        .w(px(44.))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(8.))
        .gap(px(4.));
    for (index, group) in GROUPS.iter().enumerate() {
        if index > 0 {
            rail = rail.child(div().w(px(20.)).h(px(1.)).my(px(4.)).bg(theme.border));
        }
        for (name, icon) in group.iter() {
            let spec = ControlSpec::named(Area::IconRail, Role::Button, *name);
            rail = rail.child(control(registry, spec.clone(), icon_button(&spec, *icon)));
        }
    }
    region(registry, Area::IconRail, rail)
}

#[cfg(test)]
#[path = "icon_rail_test.rs"]
mod icon_rail_test;
