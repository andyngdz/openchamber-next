//! The picker section header, its marks and the search field wrapper.

use super::list::{GAP, Header, ancestors};
use crate::icons::RemixIcon;
use crate::layout::{
    Area, ControlSpec, DrawSlot, LayoutRegistry, Role, control as control_element, placed,
};
use crate::theme::{Palette, Tokens};
use crate::views::text_metrics;
use gpui_kit::{
    AnyElement, Div, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, canvas,
    div, px,
};

/// The mark on a section caption, 12 px (`ModelPicker.dc.html` `.cap`).
const HEADER_ICON: f32 = 12.;
/// The check after the current model, `h-4 w-4` (`ModelPickerList.tsx:1063`).
const CHECK_ICON: f32 = 16.;
/// A section caption's weight, `font-weight: 500` (`ModelPicker.dc.html` `.cap`).
const HEADER_WEIGHT: f32 = 500.;
/// The longest a row's muted context may grow before it truncates, such as an agent's
/// one-line description.
const CONTEXT_MAX: f32 = 150.;
/// A footer chip's text size, `.kbd` of the mockup.
const KBD_TEXT: f32 = 11.;
/// The hint row's height.
pub(super) const FOOTER_HEIGHT: f32 = 32.;

/// A section header, recorded under its group ancestors.
pub(super) fn header(registry: &LayoutRegistry, header: &Header, palette: Palette) -> Div {
    let ancestors = ancestors(header.ancestors.iter().map(String::as_str));
    let spec =
        ControlSpec::named(Area::Popover, Role::Button, header.name.clone()).under(&ancestors);
    let muted_color: Hsla = palette.muted_foreground.into();
    // Colour carries state only: every caption mark is muted, the Favorites star included.
    let icon_color = muted_color;
    let icon_slot = record_header_mark(registry, &spec.key, header, icon_color);
    let label_slot = registry.record_text(
        &spec.key,
        crate::layout::draw::TextDraw::new(header.label.clone(), muted_color),
    );
    let line = text_metrics::line_height(text_metrics::MICRO);
    let content =
        div()
            .h(px(line))
            .flex()
            .items_center()
            .gap(px(GAP))
            .text_size(px(text_metrics::MICRO))
            .line_height(px(line))
            .font_weight(FontWeight(HEADER_WEIGHT))
            .text_color(muted_color)
            .children(header.icon.zip(icon_slot).map(|(icon, slot)| {
                placed(registry, slot, icon_box(icon, HEADER_ICON, icon_color))
            }))
            .children(
                header
                    .logo
                    .as_ref()
                    .map(|id| logo_box(id, HEADER_ICON, muted_color)),
            )
            .child(placed(
                registry,
                label_slot,
                div().child(header.label.clone()),
            ));
    control_element(registry, spec, content)
        .px(px(8.))
        .py(px(6.))
        .w_full()
}

/// Records a header's mark, a provider logo or a Remix icon, after dropping the draws of an
/// earlier render of the same header in this frame.
fn record_header_mark(
    registry: &LayoutRegistry,
    control: &SharedString,
    header: &Header,
    icon_color: Hsla,
) -> Option<DrawSlot> {
    registry.begin_draws(control);
    match (&header.logo, header.icon) {
        (Some(logo), _) => {
            record_logo(registry, control, logo);
            None
        }
        (None, Some(icon)) => {
            Some(registry.record_icon(control, crate::layout::draw::remix(icon, icon_color)))
        }
        (None, None) => None,
    }
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

/// One icon box of a row or header, at `size`.
pub(super) fn icon_box(icon: RemixIcon, size: f32, color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .text_size(px(size))
        .text_color(color)
        .child(icon)
}

/// Records a provider logo under the id the capture names it by, `img:<file stem>` for a
/// bundled logo or the models.dev id; a provider showing the fallback icon records none.
pub(super) fn record_logo(registry: &LayoutRegistry, control: &SharedString, provider_id: &str) {
    let logo = crate::icons::provider::provider_logo(provider_id);
    if let Some(id) = logo.image_id() {
        registry.record_icon(control, crate::layout::draw::image(id.to_owned()));
    }
}

/// A provider logo box, drawn from the bundled set when the provider has one.
pub(super) fn logo_box(id: &str, size: f32, muted: Hsla) -> AnyElement {
    crate::views::provider_mark::provider_mark(
        size,
        &crate::icons::provider::provider_logo(id),
        muted,
    )
}

/// A row's context label, in the muted foreground.
pub(super) fn muted(text: &str, color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .max_w(px(CONTEXT_MAX))
        .truncate()
        .text_size(px(text_metrics::MICRO))
        .line_height(px(text_metrics::line_height(text_metrics::MICRO)))
        .text_color(color)
        .child(text.to_owned())
}

/// A key of a footer hint: 11 px in the foreground tone. It has no fill: the text-fit check
/// pairs a row's same-coloured glyphs with one box, so three separate chips would fail it.
pub(super) fn kbd(tokens: &Tokens, text: &'static str) -> Div {
    div()
        .text_size(px(KBD_TEXT))
        .line_height(px(17.))
        .text_color(Hsla::from(tokens.palette.foreground))
        .child(text)
}

/// The footer hint row of a popup: `hints` as a key and its action, muted, 11.5 px.
pub(super) fn hint_row(tokens: &Tokens, hints: &[(&'static str, &'static str)]) -> Div {
    let muted: Hsla = tokens.palette.muted_foreground.into();
    let mut row = div()
        .w_full()
        .h(px(FOOTER_HEIGHT))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(12.))
        .text_size(px(11.5))
        .text_color(muted);
    for (keys, action) in hints {
        row = row.child(kbd(tokens, keys)).child(*action);
    }
    row
}

/// The 1 px divider between a popup's search, list and footer, in the `--tile2` tone.
pub(super) fn divider(tokens: &Tokens) -> Div {
    div()
        .w_full()
        .h(px(1.))
        .flex_shrink_0()
        .bg(tokens.depth.tile2)
}

/// The check after the current model.
pub(super) fn check_icon(color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .w(px(CHECK_ICON))
        .h(px(CHECK_ICON))
        .text_size(px(CHECK_ICON))
        .text_color(color)
        .child(RemixIcon::Check)
}

/// The invisible element that records a control's bounds in the frame.
pub(super) fn probe(registry: &LayoutRegistry, spec: ControlSpec) -> impl IntoElement {
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

#[cfg(test)]
#[path = "list_parts_test.rs"]
mod list_parts_test;
