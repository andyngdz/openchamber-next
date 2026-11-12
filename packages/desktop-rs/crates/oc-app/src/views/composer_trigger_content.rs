//! The content row inside a composer footer trigger, painted with the colors its control
//! records, so the dump and the frame show the same thing.

use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, Div, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, px,
};

use super::{
    GAP_AGENT, GAP_MODEL, GAP_THINKING, HOVER_OPACITY, ICON_SIZE, LABEL_WEIGHT, LOGO_SIZE,
    MODEL_LABEL_MAX, SLASH_LINE_HEIGHT, SLASH_SIZE, SLASH_WEIGHT, TriggerKind,
};
use crate::icons::RemixIcon;
use crate::icons::provider::ProviderLogo;
use crate::layout::draw::TextDraw;
use crate::layout::{ControlSpec, DrawSlot, LayoutRegistry, placed};
use crate::views::draw_image;
use crate::views::text_metrics;

/// The colors one trigger paints, the same values its control records.
#[derive(Debug, Clone, Copy)]
pub(super) struct TriggerPaint {
    /// The label run; also the row's text color.
    pub(super) label: Hsla,
    /// The Thinking trigger's slash.
    pub(super) slash: Hsla,
    /// The agent trigger's icon.
    pub(super) icon: Hsla,
}

/// What a trigger's content row paints, and the recorded draws it reports its bounds for.
pub(super) struct TriggerDraws<'a> {
    pub(super) registry: &'a LayoutRegistry,
    pub(super) paint: TriggerPaint,
    /// The Thinking slash or the agent icon.
    pub(super) lead: Option<DrawSlot>,
    pub(super) label: DrawSlot,
}

impl TriggerDraws<'_> {
    /// `painted`, reporting its bounds for `slot` when the draw was recorded.
    fn place(&self, slot: Option<DrawSlot>, painted: Div) -> Div {
        match slot {
            Some(slot) => placed(self.registry, slot, painted),
            None => painted,
        }
    }
}

/// `text-foreground` at rest; the model and agent triggers add `hover:opacity-70`
/// (`ModelControls.tsx:2358,2855`), the Thinking trigger has no hover style (`2739`).
pub(super) fn trigger_content(
    layout: TriggerKind,
    label: &str,
    label_width: f32,
    draws: &TriggerDraws<'_>,
    logo: Option<&ProviderLogo>,
) -> Div {
    let paint = draws.paint;
    let label_box = draws.place(Some(draws.label), label_box(layout, label, label_width));
    let row = div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .text_color(paint.label)
        .debug_selector(|| content_selector(layout).to_owned());
    match layout {
        TriggerKind::Model => row
            .hover(|style| style.opacity(HOVER_OPACITY))
            .gap(px(GAP_MODEL))
            .child(logo_box(LOGO_SIZE, logo))
            .child(label_box),
        TriggerKind::Thinking => row
            .gap(px(GAP_THINKING))
            .child(
                draws.place(
                    draws.lead,
                    div()
                        .text_size(px(SLASH_SIZE))
                        .line_height(px(SLASH_LINE_HEIGHT))
                        .text_color(paint.slash)
                        .child("/"),
                ),
            )
            .child(label_box),
        TriggerKind::Agent => row
            .hover(|style| style.opacity(HOVER_OPACITY))
            .gap(px(GAP_AGENT))
            .child(draws.place(
                draws.lead,
                icon_box(ICON_SIZE, RemixIcon::AiAgent, paint.icon),
            ))
            .child(label_box),
    }
}

/// The test lookup key of a trigger's content row.
pub(super) fn content_selector(layout: TriggerKind) -> &'static str {
    match layout {
        TriggerKind::Model => "composer-trigger/model",
        TriggerKind::Thinking => "composer-trigger/thinking",
        TriggerKind::Agent => "composer-trigger/agent",
    }
}

/// The test lookup key of a trigger's label box.
pub(super) fn label_selector(layout: TriggerKind) -> &'static str {
    match layout {
        TriggerKind::Model => "composer-trigger/model/label",
        TriggerKind::Thinking => "composer-trigger/thinking/label",
        TriggerKind::Agent => "composer-trigger/agent/label",
    }
}

/// The label's width: its shaped line, the model label capped at the reference's maximum,
/// rounded up to the device grid so layout never snaps the box below the line and ellipsizes it.
pub(super) fn label_width(layout: TriggerKind, label: &str, window: &mut Window, cx: &App) -> f32 {
    let shaped = crate::views::line_width(window, cx, label, text_metrics::META, LABEL_WEIGHT);
    let width = match layout {
        TriggerKind::Model => shaped.min(MODEL_LABEL_MAX),
        TriggerKind::Thinking | TriggerKind::Agent => shaped,
    };
    let scale = window.scale_factor();
    (width * scale).ceil() / scale
}

/// The trigger's width: its mark, gap and shaped label.
pub(super) fn trigger_width(
    layout: TriggerKind,
    label_width: f32,
    window: &mut Window,
    cx: &App,
) -> f32 {
    match layout {
        TriggerKind::Model => LOGO_SIZE + GAP_MODEL + label_width,
        TriggerKind::Thinking => {
            crate::views::line_width(window, cx, "/", SLASH_SIZE, SLASH_WEIGHT)
                + GAP_THINKING
                + label_width
        }
        TriggerKind::Agent => ICON_SIZE + GAP_AGENT + label_width,
    }
}

fn label_box(layout: TriggerKind, label: &str, width: f32) -> Div {
    div()
        .debug_selector(|| label_selector(layout).to_owned())
        .w(px(width))
        .flex_shrink_0()
        .truncate()
        .text_size(px(text_metrics::META))
        .line_height(px(text_metrics::line_height(text_metrics::META)))
        .font_weight(FontWeight(LABEL_WEIGHT))
        .child(label.to_owned())
}

/// The provider mark, 17 px (`ModelControls.tsx:679`); an empty box without a provider.
fn logo_box(size: f32, logo: Option<&ProviderLogo>) -> AnyElement {
    match logo {
        Some(logo) => crate::views::provider_mark::provider_mark(size, logo),
        None => div().w(px(size)).h(px(size)).into_any_element(),
    }
}

fn icon_box(size: f32, icon: RemixIcon, color: Hsla) -> Div {
    div()
        .w(px(size))
        .h(px(size))
        .text_size(px(size))
        .child(Icon::new(icon).text_color(color))
}

/// Records what [`trigger_content`] paints for this trigger, in DOM order.
pub(super) fn record_trigger<'a>(
    registry: &'a LayoutRegistry,
    spec: &ControlSpec,
    layout: TriggerKind,
    label: &str,
    paint: TriggerPaint,
    logo: Option<&ProviderLogo>,
) -> TriggerDraws<'a> {
    let lead = match layout {
        TriggerKind::Model => {
            if let Some(id) = logo.and_then(ProviderLogo::image_id) {
                draw_image(registry, spec, id.to_owned());
            }
            None
        }
        TriggerKind::Thinking => {
            Some(registry.record_text(&spec.key, TextDraw::new("/", paint.slash)))
        }
        TriggerKind::Agent => Some(registry.record_icon(
            &spec.key,
            crate::layout::draw::remix(RemixIcon::AiAgent, paint.icon),
        )),
    };
    let label = registry.record_text(&spec.key, TextDraw::new(label.to_owned(), paint.label));
    TriggerDraws {
        registry,
        paint,
        lead,
        label,
    }
}

#[cfg(test)]
#[path = "composer_trigger_content_test.rs"]
mod composer_trigger_content_test;
