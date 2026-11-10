//! The Thinking menu: Default and the model's levels, above the composer.
//!
//! The menu is sized by its content (`w-max`, `ModelControls.tsx:2747`): the footer's
//! keyboard hints are the widest row, and the panel takes their width.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, Window, div,
    px,
};

use super::common::{control, frame};
use crate::layout::Role;
use crate::views::popover::Pick;
use crate::views::{Intent, Scene};

const DEFAULT: &str = "default";
const MENU_PADDING: f32 = 4.;
const LABEL_HEIGHT: f32 = 29.;
const ROW_HEIGHT: f32 = 26.;
const FOOTER_HEIGHT: f32 = 28.;
const FOOTER_GAP: f32 = 8.;
const FOOTER_PADDING: f32 = 12.;
const NAVIGATE_HINT: &str = "↑↓ navigate";
const SWITCH_HINT: &str = "Ctrl + Shift + T switch";

/// The Thinking menu panel.
pub(super) fn panel(scene: Scene<'_>, window: &mut Window, cx: &App) -> Div {
    let width = panel_width(window, cx);
    let mut rows = vec![item_row(scene, DEFAULT, Pick::Variant(None))];
    rows.extend(
        super::selected_model_variants(scene.state)
            .into_iter()
            .map(|variant| {
                let pick = Pick::Variant(Some(variant.clone()));
                item_row(scene, &variant, pick)
            }),
    );
    let height = panel_height(scene);
    frame(
        scene,
        cx,
        Some(width),
        div()
            .flex()
            .flex_col()
            .h(px(height))
            .p(px(MENU_PADDING))
            .child(label())
            .children(rows)
            .child(footer(cx)),
    )
}

fn label() -> Div {
    div()
        .h(px(LABEL_HEIGHT))
        .px(px(8.))
        .flex()
        .items_center()
        .text_size(px(12.))
        .line_height(px(17.))
        .child("Thinking")
}

fn footer(cx: &App) -> Div {
    // `-mx-1 -mb-1 mt-1 px-3 pt-1 pb-1.5 typography-micro` (`ModelControls.tsx:2771-2775`).
    div()
        .mx(px(-MENU_PADDING))
        .mb(px(-MENU_PADDING))
        .mt(px(MENU_PADDING))
        .pt(px(4.))
        .pb(px(6.))
        .px(px(FOOTER_PADDING))
        .border_t_1()
        .border_color(cx.theme().border)
        .flex()
        .gap(px(FOOTER_GAP))
        .text_size(px(12.))
        .line_height(px(17.))
        .text_color(cx.theme().muted_foreground)
        .child(NAVIGATE_HINT)
        .child(SWITCH_HINT)
}

/// The menu's text-measured width: the two footer hints plus their gap and the CSS padding.
pub(super) fn panel_width(window: &mut Window, cx: &App) -> f32 {
    let navigate = crate::views::line_width(window, cx, NAVIGATE_HINT, 12., 400.);
    let switch = crate::views::line_width(window, cx, SWITCH_HINT, 12., 400.);
    navigate + FOOTER_GAP + switch + FOOTER_PADDING * 2.
}

/// The menu's CSS height for its current row count.
pub(super) fn panel_height(scene: Scene<'_>) -> f32 {
    let rows = 1 + super::selected_model_variants(scene.state).len();
    MENU_PADDING * 2. + LABEL_HEIGHT + rows as f32 * ROW_HEIGHT + FOOTER_HEIGHT
}

fn item_row(scene: Scene<'_>, name: &str, pick: Pick) -> Div {
    let row = div()
        .id(name.to_owned())
        .h(px(ROW_HEIGHT))
        .px(px(8.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .text_size(px(12.5))
        .line_height(px(18.))
        .child(name.to_lowercase());
    control(
        scene,
        Role::Menuitem,
        name.to_owned(),
        &[],
        row.on_click(scene.on_click_intent(Intent::Pick(pick))),
    )
}

#[cfg(test)]
#[path = "thinking_test.rs"]
mod thinking_test;
