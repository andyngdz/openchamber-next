//! Header rows and the Turn stats list of the Work status tiles.

use gpui_kit::{
    AnyElement, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};

use super::tiles::{Ink, glyph};
use super::{WORK_STATUS, button_spec};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Role, control};
use crate::views::turn_stats::{StatRow, stat_rows};
use crate::views::turns::turns;
use crate::views::{Scene, draw_icon, draw_text};

/// What a collapsible header row shows.
pub(super) struct Header {
    pub(super) name: &'static str,
    pub(super) icon: RemixIcon,
    pub(super) label: &'static str,
    pub(super) chevron: RemixIcon,
    pub(super) trailing: Option<&'static str>,
}

/// A header row: glyph, semibold label, chevron, then a muted figure at the end. It records the
/// icons and text runs the reference lists for it.
pub(super) fn header_row(scene: Scene<'_>, ink: Ink, header: Header) -> AnyElement {
    let spec = button_spec(header.name);
    let registry = scene.registry;
    draw_icon(registry, &spec, header.icon, ink.muted);
    draw_icon(registry, &spec, header.chevron, ink.muted);
    draw_text(registry, &spec, header.label, ink.foreground);
    let mut line = div()
        .id(spec.key.clone())
        .w_full()
        .h(px(30.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(glyph(header.icon, ink.muted))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(header.label))
        .child(glyph(header.chevron, ink.muted))
        .child(div().flex_1());
    if let Some(trailing) = header.trailing {
        draw_text(registry, &spec, trailing, ink.muted);
        line = line.child(div().text_color(ink.muted).child(trailing));
    }
    control(registry, spec, line).w_full().into_any_element()
}

/// A Turn stats row; React leaves it unnamed, so it is named by its visible label.
pub(super) fn finished_turn_rows(scene: Scene<'_>, ink: Ink) -> Vec<AnyElement> {
    let Some(open) = scene.state.open() else {
        return Vec::new();
    };
    let all = turns(&open.messages);
    let Some(turn) = all.last() else {
        return Vec::new();
    };
    stat_rows(turn)
        .into_iter()
        .map(|row| stat_row(scene, row, ink))
        .collect()
}

/// A 28 px description-list row: the muted key at the start, its figure at the end.
pub(super) fn stat_row(scene: Scene<'_>, row: StatRow, ink: Ink) -> AnyElement {
    let spec =
        ControlSpec::named(Area::WorkStatusPanel, Role::Generic, row.label).under(&[WORK_STATUS]);
    let line = div()
        .id(spec.key.clone())
        .w_full()
        .h(px(28.))
        .flex()
        .items_center()
        .child(div().flex_1().text_color(ink.muted).child(row.label))
        .child(row.value);
    control(scene.registry, spec, line)
        .w_full()
        .into_any_element()
}

#[cfg(test)]
#[path = "work_status_rows_test.rs"]
mod work_status_rows_test;
