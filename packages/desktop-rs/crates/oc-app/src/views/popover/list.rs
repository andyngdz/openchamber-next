//! Shared rows for the picker lists: their specs, their recorded colors and their content.

use super::Pick;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, DrawSlot, LayoutRegistry, Role, placed};
use crate::views::text_metrics;
use gpui_kit::component::list::ListItem;
use gpui_kit::{App, Div, FontWeight, Hsla, IntoElement, ParentElement, Styled, div, px};

use super::list_parts::{check_icon, icon_box, logo_box, muted, probe, record_logo};

/// The provider mark on a Favorites or Recent row, `size-3.5` (`ModelPickerList.tsx:1054`).
const ROW_ICON: f32 = 14.;
/// `gap-1.5` between a row's mark, name, context and check (`ModelPickerList.tsx:1052`).
pub(super) const GAP: f32 = 6.;
/// `font-medium` on a model name (`ModelPickerList.tsx:1055`).
const ROW_WEIGHT: f32 = 500.;

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
    /// A provider logo instead of a Remix icon, as `img:<id>`.
    pub(super) logo: Option<String>,
    /// An agent's color dot, drawn as a 4 px mark.
    pub(super) dot: Option<gpui_kit::Hsla>,
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
    /// A provider logo instead of a Remix icon, as `img:<id>`.
    pub(super) logo: Option<String>,
    pub(super) ancestors: Vec<String>,
}

pub(super) use super::list_parts::{header, search};

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
    let spec = row_spec(row);
    let paint = row_paint(cx);
    let slots = record_row(registry, &spec, row, paint);
    let content = row_content(registry, spec, row, paint, &slots);
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

/// The spec a row records under: its ancestors, role and name, plus any description.
fn row_spec(row: &Row) -> ControlSpec {
    let ancestors = ancestors(row.ancestors.iter().map(String::as_str));
    let mut spec = ControlSpec::named(Area::Popover, row.role, row.name.clone()).under(&ancestors);
    if let Some(description) = row.described {
        spec = spec.described(description);
    }
    if let Some(index) = row.repeat {
        spec = spec.repeated(index);
    }
    spec
}

/// The colors one row paints, the same values it records.
#[derive(Debug, Clone, Copy)]
struct RowPaint {
    /// The model name, in the component's list row text color.
    label: Hsla,
    /// The mark and the context, `text-muted-foreground`.
    muted: Hsla,
    /// The check after the current model, `text-foreground`.
    check: Hsla,
}

/// Every color a row draws, read once from the palette and component style.
fn row_paint(cx: &App) -> RowPaint {
    let palette = crate::theme::palette();
    RowPaint {
        label: crate::layout::ComponentStyle::read(cx).list_row_text,
        muted: palette.muted_foreground.into(),
        check: palette.foreground.into(),
    }
}

/// Where a row's recorded draws sit, so the elements that paint them report their bounds.
struct RowSlots {
    icon: Option<DrawSlot>,
    label: DrawSlot,
    context: Option<DrawSlot>,
    check: Option<DrawSlot>,
}

/// Records every draw of one row, in DOM order.
fn record_row(
    registry: &LayoutRegistry,
    spec: &ControlSpec,
    row: &Row,
    paint: RowPaint,
) -> RowSlots {
    registry.begin_draws(&spec.key);
    let icon = match (&row.logo, row.icon) {
        (Some(logo), _) => {
            record_logo(registry, &spec.key, logo);
            None
        }
        (None, Some(icon)) => {
            Some(registry.record_icon(&spec.key, crate::layout::draw::remix(icon, paint.muted)))
        }
        (None, None) => None,
    };
    let label = registry.record_text(
        &spec.key,
        crate::layout::draw::TextDraw::new(row.label.clone(), paint.label),
    );
    let context = row.context.as_ref().map(|context| {
        registry.record_text(
            &spec.key,
            crate::layout::draw::TextDraw::new(context.clone(), paint.muted),
        )
    });
    let check = row.selected.then(|| {
        registry.record_icon(
            &spec.key,
            crate::layout::draw::remix(RemixIcon::Check, paint.check),
        )
    });
    RowSlots {
        icon,
        label,
        context,
        check,
    }
}

/// The row's content, painted with the colors [`record_row`] recorded for it.
fn row_content(
    registry: &LayoutRegistry,
    spec: ControlSpec,
    row: &Row,
    paint: RowPaint,
    slots: &RowSlots,
) -> Div {
    let dot = row
        .dot
        .map(|color| crate::layout::mark(registry, &spec.key, color, px(4.), px(4.)));
    let mark = match (&row.logo, row.icon, slots.icon) {
        (Some(id), _, _) => Some(logo_box(id, ROW_ICON)),
        (None, Some(icon), Some(slot)) => {
            Some(placed(registry, slot, icon_box(icon, ROW_ICON, paint.muted)).into_any_element())
        }
        _ => None,
    };
    let label = div()
        .flex_1()
        .min_w_0()
        .truncate()
        .font_weight(FontWeight(ROW_WEIGHT))
        .child(row.label.clone());
    div()
        .relative()
        .w_full()
        .flex()
        .items_center()
        .gap(px(GAP))
        .child(probe(registry, spec))
        .children(dot)
        .children(mark)
        .child(placed(registry, slots.label, label))
        .children(
            row.context
                .as_ref()
                .zip(slots.context)
                .map(|(context, slot)| placed(registry, slot, muted(context, paint.muted))),
        )
        .children(
            slots
                .check
                .map(|slot| placed(registry, slot, check_icon(paint.check))),
        )
}

/// The named ancestors, outermost first, with empty names left out.
pub(super) fn ancestors<'a>(parts: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    parts.filter(|part| !part.is_empty()).collect()
}

#[cfg(test)]
#[path = "list_test.rs"]
mod list_test;
