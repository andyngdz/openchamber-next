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
use crate::views::text_metrics;
use crate::views::{Intent, Scene};

const DEFAULT: &str = "default";
/// `p-1` on the popup (`dropdown-menu.styles.ts:1`): 0.25rem.
const MENU_PADDING: f32 = 4.;
/// `py-1.5` on the label (`MobileSheetList.tsx:11`): 0.375rem.
const LABEL_PADDING_Y: f32 = 6.;
/// `py-1` on an item (`dropdown-menu.styles.ts`): 0.25rem.
const ITEM_PADDING_Y: f32 = 4.;
/// `gap-x-2` between the two footer hints (`ModelControls.tsx:2772`): 0.5rem.
const FOOTER_GAP: f32 = 8.;
/// `px-3` on the footer (`ModelControls.tsx:2772`): 0.75rem.
const FOOTER_PADDING: f32 = 12.;
/// `pt-1` / `pb-1.5` on the footer (`ModelControls.tsx:2772`): 0.25rem / 0.375rem.
const FOOTER_PADDING_TOP: f32 = 4.;
const FOOTER_PADDING_BOTTOM: f32 = 6.;
/// `border-t` on the footer (`ModelControls.tsx:2772`): 1px.
const BORDER_TOP: f32 = 1.;
/// `px-2` on a row and the label (`MobileSheetList.tsx:11`): 0.5rem.
const ROW_PADDING_X: f32 = 8.;
/// `rounded-lg` (`dropdown-menu.styles.ts` item) = `--radius-lg` 0.5625rem
/// (`design-system.css:893`).
const ROW_RADIUS: f32 = 9.;
const NAVIGATE_HINT: &str = "↑↓ navigate";
const SWITCH_HINT: &str = "Ctrl + Shift + T switch";

/// `px-2 py-1.5 typography-micro` label (`MobileSheetList.tsx:11`): 12 + 17.
fn label_height() -> f32 {
    2. * LABEL_PADDING_Y + text_metrics::line_height(text_metrics::MICRO)
}

/// `px-2 py-1` item with `typography-meta` (`dropdown-menu.styles.ts`, `ModelControls.tsx:2749`):
/// 4 + 18 + 4.
fn row_height() -> f32 {
    2. * ITEM_PADDING_Y + text_metrics::line_height(text_metrics::META)
}

/// `pt-1 pb-1.5 border-t typography-micro` (`ModelControls.tsx:2772`): 4 + 6 + 1 + 17.
fn footer_height() -> f32 {
    FOOTER_PADDING_TOP
        + FOOTER_PADDING_BOTTOM
        + BORDER_TOP
        + text_metrics::line_height(text_metrics::MICRO)
}

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
        .h(px(label_height()))
        .px(px(ROW_PADDING_X))
        .flex()
        .items_center()
        .text_size(px(text_metrics::MICRO))
        .line_height(px(text_metrics::line_height(text_metrics::MICRO)))
        .child("Thinking")
}

fn footer(cx: &App) -> Div {
    // `-mx-1 -mb-1 mt-1 px-3 pt-1 pb-1.5 typography-micro` (`ModelControls.tsx:2771-2775`).
    div()
        .mx(px(-MENU_PADDING))
        .mb(px(-MENU_PADDING))
        .mt(px(MENU_PADDING))
        .pt(px(FOOTER_PADDING_TOP))
        .pb(px(FOOTER_PADDING_BOTTOM))
        .px(px(FOOTER_PADDING))
        .border_t_1()
        .border_color(cx.theme().border)
        .flex()
        .gap(px(FOOTER_GAP))
        .text_size(px(text_metrics::MICRO))
        .line_height(px(text_metrics::line_height(text_metrics::MICRO)))
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
    // The footer's `mt-1` and `-mb-1` cancel (`ModelControls.tsx:2772`), so it adds
    // its border-box height to the panel's content.
    MENU_PADDING * 2. + label_height() + rows as f32 * row_height() + footer_height()
}

fn item_row(scene: Scene<'_>, name: &str, pick: Pick) -> Div {
    let row = div()
        .id(name.to_owned())
        .h(px(row_height()))
        .px(px(ROW_PADDING_X))
        .rounded(px(ROW_RADIUS))
        .flex()
        .items_center()
        .text_size(px(text_metrics::META))
        .line_height(px(text_metrics::line_height(text_metrics::META)))
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
