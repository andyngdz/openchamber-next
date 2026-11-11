//! The composer footer's left icon group and right trigger/action group.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Disableable};
use gpui_kit::{
    AnyElement, App, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, px,
};
use oc_state::{ComposerStatus, ConnectionStatus, State};

use super::slots::{Tool, TriggerKind, tool_name};
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};
use crate::views::popover::trigger_control;
use crate::views::text_metrics;
use crate::views::{Intent, Scene, icon_button};

/// `h-8` (`ModelControls.tsx:677`): 2rem.
const TRIGGER_HEIGHT: f32 = 32.;
/// `providerLogoSize size-[17px]` (`ModelControls.tsx:679`).
const LOGO_SIZE: f32 = 17.;
/// `controlIconSize size-4` (`ModelControls.tsx:678`).
const ICON_SIZE: f32 = 16.;
/// `gap-1.5` for the model/agent labels (`ModelControls.tsx:2358,2855`): 0.375rem.
const GAP_MODEL: f32 = 6.;
/// `gap-1` between the Thinking slash and label (`ModelControls.tsx:2739`): 0.25rem.
const GAP_THINKING: f32 = 4.;
/// `gap-1.5` on the agent trigger (`ModelControls.tsx:2855`): 0.375rem.
const GAP_AGENT: f32 = 6.;
/// `gap-x-3` between the footer's flex children (`ComposerFooter.tsx:284-288`): 0.75rem.
const GAP_GROUP: f32 = 12.;
/// The first footer group's `gap-x-1.5` (`ComposerFooter.tsx:264`): 0.375rem.
const GAP_LEFT: f32 = 6.;
/// `font-medium` on the trigger label (`ModelControls.tsx:2358,2739,2855`).
const LABEL_WEIGHT: f32 = 500.;
/// The slash is an unstyled span, so it inherits the interface base size and 1.5 leading:
/// 16px / 24px (`design-system.css:210-214` sets only family and weight).
const SLASH_SIZE: f32 = 16.;
const SLASH_LINE_HEIGHT: f32 = 24.;
const SLASH_WEIGHT: f32 = 400.;
/// `max-w-[260px]` on the model label (`ModelControls.tsx:2396`).
const MODEL_LABEL_MAX: f32 = 260.;
/// `hover:opacity-70` on the model and agent triggers (`ModelControls.tsx:2358,2855`).
const HOVER_OPACITY: f32 = 0.7;

/// The left icon group.
pub(super) fn left_group(tools: &[Tool], registry: &LayoutRegistry) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(GAP_LEFT))
        .children(tools.iter().map(|tool| match tool {
            Tool::Icon(name, icon) => icon_control(name, *icon, registry),
            _ => div().into_any_element(),
        }))
}

/// The right trigger and action group.
pub(super) fn right_group(
    scene: Scene<'_>,
    tools: &[Tool],
    registry: &LayoutRegistry,
    window: &mut Window,
    cx: &App,
) -> Div {
    let mut group = div().flex().items_center();
    for (index, tool) in tools.iter().enumerate() {
        let slot = match tool {
            Tool::Trigger { .. } => trigger(scene, tool, window, cx),
            Tool::Icon(name, icon) => icon_control(name, *icon, registry),
            Tool::Send | Tool::Stop => action_control(scene, tool, registry),
        };
        let gap = if index == 0 {
            0.
        } else {
            match tool {
                Tool::Trigger {
                    layout: TriggerKind::Thinking,
                    ..
                } => GAP_THINKING,
                _ => GAP_GROUP,
            }
        };
        group = group.child(div().ml(px(gap)).child(slot));
    }
    group
}

fn icon_control(name: &str, icon: IconName, registry: &LayoutRegistry) -> AnyElement {
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    control(registry, spec.clone(), icon_button(&spec, icon)).into_any_element()
}

fn action_control(scene: Scene<'_>, tool: &Tool, registry: &LayoutRegistry) -> AnyElement {
    let name = tool_name(tool).unwrap_or_default();
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    let button = match tool {
        Tool::Stop => icon_button(&spec, IconName::Square),
        _ => icon_button(&spec, IconName::ArrowUp)
            .disabled(!can_send(scene.state))
            .on_click(scene.on_click_intent(Intent::SendPrompt)),
    };
    control(registry, spec, button).into_any_element()
}

fn trigger(scene: Scene<'_>, tool: &Tool, window: &mut Window, cx: &App) -> AnyElement {
    let Tool::Trigger {
        kind,
        layout,
        name,
        label,
    } = tool
    else {
        return div().into_any_element();
    };
    let spec = ControlSpec::named(Area::Composer, Role::Button, name.clone());
    let label_width = label_width(*layout, label, window, cx);
    let width = trigger_width(*layout, label_width, window, cx);
    // `text` is gpui-kit's padless button with no background. A padded variant clips the label,
    // because the width below holds only the content. Its own text colors dim at rest and
    // brighten on hover, so the content row sets the reference's colors itself.
    let button = Button::new(spec.key.clone())
        .text()
        .w(px(width))
        .h(px(TRIGGER_HEIGHT))
        .accessibility_label(name.clone())
        .child(trigger_content(*layout, label, label_width, cx));
    trigger_control(scene, *kind, button, cx).into_any_element()
}

/// The label's width: its shaped line, the model label capped at the reference's maximum,
/// rounded up to the device grid so layout never snaps the box below the line and ellipsizes it.
fn label_width(layout: TriggerKind, label: &str, window: &mut Window, cx: &App) -> f32 {
    let shaped = crate::views::line_width(window, cx, label, text_metrics::META, LABEL_WEIGHT);
    let width = match layout {
        TriggerKind::Model => shaped.min(MODEL_LABEL_MAX),
        TriggerKind::Thinking | TriggerKind::Agent => shaped,
    };
    let scale = window.scale_factor();
    (width * scale).ceil() / scale
}

/// `text-foreground` at rest; the model and agent triggers add `hover:opacity-70`
/// (`ModelControls.tsx:2358,2855`), the Thinking trigger has no hover style (`2739`).
fn trigger_content(layout: TriggerKind, label: &str, label_width: f32, cx: &App) -> Div {
    let row = div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .text_color(cx.theme().foreground)
        .debug_selector(|| content_selector(layout).to_owned());
    match layout {
        TriggerKind::Model => row
            .hover(|style| style.opacity(HOVER_OPACITY))
            .gap(px(GAP_MODEL))
            .child(icon_box(LOGO_SIZE, IconName::Bot))
            .child(label_box(layout, label, label_width)),
        TriggerKind::Thinking => row
            .gap(px(GAP_THINKING))
            .child(
                div()
                    .text_size(px(SLASH_SIZE))
                    .line_height(px(SLASH_LINE_HEIGHT))
                    .child("/"),
            )
            .child(label_box(layout, label, label_width)),
        TriggerKind::Agent => row
            .hover(|style| style.opacity(HOVER_OPACITY))
            .gap(px(GAP_AGENT))
            .child(icon_box(ICON_SIZE, IconName::CircleUser))
            .child(label_box(layout, label, label_width)),
    }
}

/// Test lookup keys for a trigger's content row and its label.
fn content_selector(layout: TriggerKind) -> &'static str {
    match layout {
        TriggerKind::Model => "composer-trigger/model",
        TriggerKind::Thinking => "composer-trigger/thinking",
        TriggerKind::Agent => "composer-trigger/agent",
    }
}

fn label_selector(layout: TriggerKind) -> &'static str {
    match layout {
        TriggerKind::Model => "composer-trigger/model/label",
        TriggerKind::Thinking => "composer-trigger/thinking/label",
        TriggerKind::Agent => "composer-trigger/agent/label",
    }
}

fn trigger_width(layout: TriggerKind, label_width: f32, window: &mut Window, cx: &App) -> f32 {
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

fn icon_box(size: f32, icon: IconName) -> Div {
    div()
        .w(px(size))
        .h(px(size))
        .text_size(px(size))
        .child(icon)
}

fn can_send(state: &State) -> bool {
    state.connection() == &ConnectionStatus::Connected
        && state.composer().status != ComposerStatus::Sending
        && !state.composer().draft.trim().is_empty()
}

#[cfg(test)]
#[path = "composer_controls_test.rs"]
mod composer_controls_test;
