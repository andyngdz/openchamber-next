//! The icon rail on the right edge: panel switches, all inert in the spike.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{App, Div, Hsla, IntoElement, ParentElement, Styled, div, px};

use super::draw_icon;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control, region};
use crate::theme::{DepthStyle, Palette, Tokens, palette};

/// The rail's buttons in reference order, in groups split by a divider.
const GROUPS: [&[(&str, RemixIcon)]; 5] = [
    &[("Context", RemixIcon::DonutChartFill)],
    &[
        ("Git", RemixIcon::GitBranch),
        ("Changes", RemixIcon::LayoutColumn),
        ("Walkthrough", RemixIcon::Route),
    ],
    &[
        ("Files", RemixIcon::FileEdit),
        ("Terminal", RemixIcon::TerminalBox),
        ("Browser", RemixIcon::Global),
    ],
    &[("Project knowledge", RemixIcon::BookMarked)],
    &[("Configure panels", RemixIcon::Equalizer2)],
];

/// The panel the rail shows as open: the Work status panel is the Context one.
const ACTIVE: &str = "Context";
/// The button that sits at the bottom of the rail, below the spacer.
const CONFIGURE: &str = "Configure panels";
const BUTTON: f32 = 32.;
const BUTTON_RADIUS: f32 = 8.;
const ICON: f32 = 17.;

/// The 44 px rail.
pub(crate) fn icon_rail(registry: &LayoutRegistry, cx: &App) -> Div {
    let tokens = Tokens::of(cx);
    let colors = palette(cx);
    let mut rail = div()
        .w(px(44.))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(6.))
        .pb(px(10.))
        .gap(px(4.));
    for (index, group) in GROUPS.iter().enumerate() {
        if index > 0 && index + 1 < GROUPS.len() {
            rail = rail.child(div().w(px(16.)).h(px(1.)).my(px(6.)).bg(tokens.depth.tile2));
        }
        for (name, icon) in group.iter() {
            if *name == CONFIGURE {
                rail = rail.child(div().flex_1());
            }
            rail = rail.child(rail_button(registry, tokens, colors, name, *icon));
        }
    }
    region(registry, Area::IconRail, rail)
}

/// One 32 px button: muted at rest, a raised control with the foreground icon when active.
fn rail_button(
    registry: &LayoutRegistry,
    tokens: &Tokens,
    colors: Palette,
    name: &'static str,
    icon: RemixIcon,
) -> impl IntoElement {
    let spec = ControlSpec::named(Area::IconRail, Role::Button, name);
    let active = name == ACTIVE;
    let color: Hsla = if active {
        colors.foreground.into()
    } else {
        colors.muted_foreground.into()
    };
    draw_icon(registry, &spec, icon, color);
    let button = Button::new(spec.key.clone())
        .ghost()
        .small()
        .w(px(BUTTON))
        .h(px(BUTTON))
        .rounded(px(BUTTON_RADIUS))
        .text_color(color)
        .icon(Icon::new(icon).with_size(px(ICON)))
        .tooltip(spec.name.clone());
    let frame = div().size(px(BUTTON)).rounded(px(BUTTON_RADIUS));
    let frame = if active {
        frame.raised_control(tokens).rounded(px(BUTTON_RADIUS))
    } else {
        frame
    };
    control(registry, spec, frame.child(button))
}

#[cfg(test)]
#[path = "icon_rail_test.rs"]
mod icon_rail_test;
