//! Shared rows, section headers and the recorded search field for the picker lists.

use gpui_kit::component::list::ListItem;
use gpui_kit::{
    App, Div, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, canvas, div, px,
};
use oc_state::SelectionAction;

use super::Pick;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Reveal, Role, control as control_element};
use crate::views::{Intent, text_metrics};

/// One row's action when the list confirms it.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum RowAction {
    /// Pick this row's value.
    Pick(Pick),
    /// Close the popup, as React's non-pick rows do.
    Close,
}

/// Which star a row shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StarKind {
    /// The model or agent is starred.
    Filled,
    /// The row can be starred.
    Outline,
}

/// A row's star toggle.
pub(super) struct Star {
    pub(super) label: &'static str,
    pub(super) action: SelectionAction,
    pub(super) kind: StarKind,
    pub(super) shown: bool,
}

/// One list row, named the way the reference keys it.
pub(super) struct Row {
    pub(super) name: String,
    pub(super) visible: String,
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

/// The button-shaped group header the reference lists.
pub(super) struct Header {
    pub(super) name: String,
    pub(super) ancestors: Vec<String>,
}

/// A row as the list draws it, with its accessible name and recorded bounds.
pub(super) fn row_item(
    registry: &LayoutRegistry,
    dispatch: &super::Dispatch,
    row: &Row,
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
        .child(probe(registry, spec))
        .child(row.visible.clone());
    let mut list_item = ListItem::new(row.name.clone())
        .accessibility_label(row.name.clone())
        .px(px(8.))
        .py(px(8.))
        .text_size(px(text_metrics::META))
        .line_height(px(text_metrics::line_height(text_metrics::META)))
        .child(content);
    if let Some(star) = &row.star {
        list_item = list_item.suffix(star_suffix(registry.clone(), dispatch.clone(), row, star));
    }
    list_item
}

fn star_suffix(
    registry: LayoutRegistry,
    dispatch: super::Dispatch,
    row: &Row,
    star: &Star,
) -> impl Fn(&mut Window, &mut App) -> Div + 'static {
    let mut parts: Vec<&str> = row.ancestors.iter().map(String::as_str).collect();
    if row.star_under_row {
        parts.push(row.name.as_str());
    }
    let ancestors = ancestors(parts.into_iter());
    let label = star.label;
    let action = star.action.clone();
    let kind = star.kind;
    let mut spec = ControlSpec::named(Area::Popover, Role::Button, label).under(&ancestors);
    if let Some(index) = row.repeat {
        spec = spec.repeated(index);
    }
    let spec = if star.shown {
        spec
    } else {
        spec.revealed(Reveal::Hidden)
    };
    move |_, _| {
        let dispatch = dispatch.clone();
        let action = action.clone();
        control_element(
            &registry,
            spec.clone(),
            div()
                .id(label)
                .w(px(16.))
                .h(px(16.))
                .child(match kind {
                    StarKind::Filled => gpui_kit::assets::IconName::StarFill,
                    StarKind::Outline => gpui_kit::assets::IconName::Star,
                })
                .on_click(move |_, window, cx| {
                    dispatch(&Intent::ApplySelection(action.clone()), window, cx);
                    cx.stop_propagation();
                }),
        )
        .w(px(16.))
        .h(px(16.))
    }
}

/// A section header, recorded under its group ancestors.
pub(super) fn header(registry: &LayoutRegistry, header: &Header) -> Div {
    let ancestors = ancestors(header.ancestors.iter().map(String::as_str));
    let spec =
        ControlSpec::named(Area::Popover, Role::Button, header.name.clone()).under(&ancestors);
    let content = div()
        .h(px(text_metrics::line_height(text_metrics::MICRO)))
        .flex()
        .items_center()
        .text_size(px(text_metrics::MICRO))
        .line_height(px(text_metrics::line_height(text_metrics::MICRO)))
        .child(header.name.clone());
    control_element(registry, spec, content).px(px(8.)).w_full()
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

fn ancestors<'a>(parts: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    parts.filter(|part| !part.is_empty()).collect()
}

#[cfg(test)]
#[path = "list_test.rs"]
mod list_test;
