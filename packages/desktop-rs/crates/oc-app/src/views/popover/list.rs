//! Shared rows, section headers and the recorded search field for the picker lists.

use super::Pick;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control as control_element};
use crate::views::text_metrics;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::list::ListItem;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{App, Div, FontWeight, IntoElement, ParentElement, Styled, canvas, div, px};

/// The provider mark on a Favorites or Recent row, `size-3.5` (`ModelPickerList.tsx:1054`).
const ROW_ICON: f32 = 14.;
/// The provider mark on a section header, `h-4 w-4` (`ModelPickerList.tsx:1104`).
const HEADER_ICON: f32 = 16.;
/// The check after the current model, `h-4 w-4` (`ModelPickerList.tsx:1063`).
const CHECK_ICON: f32 = 16.;
/// `gap-1.5` between a row's mark, name, context and check (`ModelPickerList.tsx:1052`).
const GAP: f32 = 6.;
/// `font-medium` on a model name (`ModelPickerList.tsx:1055`).
const ROW_WEIGHT: f32 = 500.;
/// `font-semibold` on a section label (`ModelPickerList.tsx:942`).
const HEADER_WEIGHT: f32 = 600.;

/// One row's action when the list confirms it.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum RowAction {
    /// Pick this row's value.
    Pick(Pick),
    /// Close the popup, as React's non-pick rows do.
    Close,
}

pub(super) use super::star::{Star, StarKind};

/// One list row: its accessible name, what it draws, and its action.
pub(super) struct Row {
    pub(super) name: String,
    pub(super) label: String,
    pub(super) icon: Option<RemixIcon>,
    pub(super) context: Option<String>,
    pub(super) selected: bool,
    pub(super) role: Role,
    pub(super) ancestors: Vec<String>,
    pub(super) action: RowAction,
    pub(super) star: Option<Star>,
    pub(super) described: Option<&'static str>,
    pub(super) repeat: Option<usize>,
    pub(super) star_under_row: bool,
}

/// A section header and the rows under it.
pub(super) struct Section {
    pub(super) header: Option<Header>,
    pub(super) rows: Vec<Row>,
}

/// The group header the reference lists; its label is drawn, its name is accessible.
pub(super) struct Header {
    pub(super) name: String,
    pub(super) label: String,
    pub(super) icon: Option<RemixIcon>,
    pub(super) ancestors: Vec<String>,
}

/// A row as the list draws it, with its accessible name and recorded bounds.
///
/// React names the row from its visible parts and leaves the provider visual out; here the
/// accessible name stays exactly as the reference keys it while the row draws only its mark,
/// model name, context, check and star.
pub(super) fn row_item(
    registry: &LayoutRegistry,
    dispatch: &super::Dispatch,
    row: &Row,
    cx: &App,
) -> ListItem {
    let ancestors = ancestors(row.ancestors.iter().map(String::as_str));
    let mut spec = ControlSpec::named(Area::Popover, row.role, row.name.clone()).under(&ancestors);
    if let Some(description) = row.described {
        spec = spec.described(description);
    }
    if let Some(index) = row.repeat {
        spec = spec.repeated(index);
    }
    let content = div()
        .relative()
        .w_full()
        .flex()
        .items_center()
        .gap(px(GAP))
        .child(probe(registry, spec))
        .children(row.icon.map(|icon| icon_box(icon, ROW_ICON, cx)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight(ROW_WEIGHT))
                .child(row.label.clone()),
        )
        .children(row.context.as_ref().map(|context| muted(context, cx)))
        .when(row.selected, |this| this.child(check_icon()));
    let mut list_item = ListItem::new(row.name.clone())
        .accessibility_label(row.name.clone())
        .px(px(8.))
        .py(px(6.))
        .text_size(px(text_metrics::META))
        .line_height(px(text_metrics::line_height(text_metrics::META)))
        .child(content);
    if let Some(star) = &row.star {
        list_item = list_item.suffix(super::star::suffix(
            registry.clone(),
            dispatch.clone(),
            row,
            star,
        ));
    }
    list_item
}

/// A section header, recorded under its group ancestors.
pub(super) fn header(registry: &LayoutRegistry, header: &Header, cx: &App) -> Div {
    let ancestors = ancestors(header.ancestors.iter().map(String::as_str));
    let spec =
        ControlSpec::named(Area::Popover, Role::Button, header.name.clone()).under(&ancestors);
    let line = text_metrics::line_height(text_metrics::MICRO);
    let content = div()
        .h(px(line))
        .flex()
        .items_center()
        .gap(px(GAP))
        .text_size(px(text_metrics::MICRO))
        .line_height(px(line))
        .font_weight(FontWeight(HEADER_WEIGHT))
        .text_color(cx.theme().muted_foreground)
        .children(header.icon.map(|icon| icon_box(icon, HEADER_ICON, cx)))
        .child(header.label.clone());
    control_element(registry, spec, content)
        .px(px(8.))
        .py(px(6.))
        .w_full()
}

/// The list with the search field, recorded under the reference's search key.
pub(super) fn search(
    registry: &LayoutRegistry,
    role: Role,
    placeholder: &str,
    list: impl IntoElement,
) -> Div {
    let spec = ControlSpec::named(Area::Popover, role, placeholder);
    control_element(registry, spec, list).w_full()
}

fn icon_box(icon: RemixIcon, size: f32, cx: &App) -> Div {
    div()
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .text_size(px(size))
        .text_color(cx.theme().muted_foreground)
        .child(icon)
}

fn muted(text: &str, cx: &App) -> Div {
    div()
        .flex_shrink_0()
        .text_size(px(text_metrics::MICRO))
        .line_height(px(text_metrics::line_height(text_metrics::MICRO)))
        .text_color(cx.theme().muted_foreground)
        .child(text.to_owned())
}

fn check_icon() -> Div {
    div()
        .flex_shrink_0()
        .w(px(CHECK_ICON))
        .h(px(CHECK_ICON))
        .text_size(px(CHECK_ICON))
        .child(RemixIcon::Check)
}

fn probe(registry: &LayoutRegistry, spec: ControlSpec) -> impl IntoElement {
    let registry = registry.clone();
    canvas(
        move |bounds, _, _| registry.record_control(spec.clone(), bounds),
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The named ancestors, outermost first, with empty names left out.
pub(super) fn ancestors<'a>(parts: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    parts.filter(|part| !part.is_empty()).collect()
}

#[cfg(test)]
#[path = "list_test.rs"]
mod list_test;
