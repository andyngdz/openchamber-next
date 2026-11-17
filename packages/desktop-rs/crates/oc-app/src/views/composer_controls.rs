//! The composer footer's left icon group and right trigger/action group.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Disableable, Sizable};
use gpui_kit::{AnyElement, App, Div, IntoElement, ParentElement, Styled, Window, div, px};
use oc_state::{ComposerStatus, ConnectionStatus, State};

use super::slots::{Tool, TriggerKind, tool_name};
use trigger_content::{TriggerPaint, label_width, record_trigger, trigger_content, trigger_width};
#[cfg(test)]
use trigger_content::{content_selector, label_selector};

#[path = "composer_trigger_content.rs"]
mod trigger_content;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};
use crate::theme::{DepthStyle, Palette, Tokens};
use crate::views::popover::trigger_control;
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
const GAP_LEFT: f32 = 2.;
/// The icon buttons are 28 square (the mockup's footer), the send button round.
const BUTTON_SIZE: f32 = 28.;
const SEND_RADIUS: f32 = 14.;
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
pub(super) fn left_group(scene: Scene<'_>, tools: &[Tool], registry: &LayoutRegistry) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(GAP_LEFT))
        .children(tools.iter().map(|tool| match tool {
            Tool::Icon(name, icon) => icon_control(scene, name, *icon, registry),
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
            Tool::Icon(name, icon) => icon_control(scene, name, *icon, registry),
            Tool::Send | Tool::Stop => action_control(scene, tool, registry, cx),
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

fn icon_control(
    scene: Scene<'_>,
    name: &str,
    icon: RemixIcon,
    registry: &LayoutRegistry,
) -> AnyElement {
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    let muted = scene.palette.muted_foreground.into();
    control(
        registry,
        spec.clone(),
        icon_button(registry, &spec, icon, muted)
            .w(px(BUTTON_SIZE))
            .h(px(BUTTON_SIZE)),
    )
    .into_any_element()
}

fn action_control(
    scene: Scene<'_>,
    tool: &Tool,
    registry: &LayoutRegistry,
    cx: &App,
) -> AnyElement {
    let name = tool_name(tool).unwrap_or_default();
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    let palette = scene.palette;
    let tokens = Tokens::of(cx);
    let enabled = can_send(scene.state);
    let button = match tool {
        // React's own `StopIcon` (`components/icons/StopIcon.tsx`), in the foreground on the
        // strong tile while a turn runs.
        Tool::Stop => {
            let color: gpui_kit::Hsla = palette.foreground.into();
            registry.record_icon(&spec.key, crate::layout::draw::IconDraw::Custom { color });
            Button::new(spec.key.clone())
                .ghost()
                .small()
                .text_color(color)
                .icon(gpui_kit::component::Icon::new(
                    crate::icons::CustomIcon::Stop,
                ))
                .tooltip(spec.name.clone())
                .bg(tokens.depth.tile2)
        }
        // Primary fill when there is text to send, the strong tile otherwise.
        _ => {
            let color = if enabled {
                palette.primary_foreground.into()
            } else {
                palette.muted_foreground.into()
            };
            let send = icon_button(registry, &spec, RemixIcon::SendPlane2, color)
                .disabled(!enabled)
                .on_click(scene.on_click_intent(Intent::SendPrompt));
            if enabled {
                send.raised_control(tokens)
                    .bg(gpui_kit::Hsla::from(palette.primary))
            } else {
                send.bg(tokens.depth.tile2)
            }
        }
    }
    .w(px(BUTTON_SIZE))
    .h(px(BUTTON_SIZE))
    .rounded(px(SEND_RADIUS));
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
    let paint = trigger_paint(scene.state, scene.palette, *layout);
    let logo = match layout {
        TriggerKind::Model => crate::views::popover::selected_model(scene.state)
            .map(|(provider_id, _)| crate::icons::provider::provider_logo(&provider_id)),
        TriggerKind::Thinking | TriggerKind::Agent => None,
    };
    let draws = record_trigger(scene.registry, &spec, *layout, label, paint, logo.as_ref());
    // The Thinking label carries `lowercase` (`ModelControls.tsx:2678`); the capture records
    // the text node, so the record keeps the original case.
    let shown = match layout {
        TriggerKind::Thinking => label.to_lowercase(),
        TriggerKind::Model | TriggerKind::Agent => label.clone(),
    };
    let label_width = label_width(*layout, &shown, window, cx);
    let width = trigger_width(*layout, label_width, window, cx);
    // `text` is gpui-kit's padless button with no background. A padded variant clips the label,
    // because the width below holds only the content. Its own text colors dim at rest and
    // brighten on hover, so the content row paints the reference's colors itself.
    let button = Button::new(spec.key.clone())
        .text()
        .w(px(width))
        .h(px(TRIGGER_HEIGHT))
        .accessibility_label(name.clone())
        .child(trigger_content(
            *layout,
            &shown,
            label_width,
            &draws,
            logo.as_ref(),
        ));
    trigger_control(scene, *kind, button, cx).into_any_element()
}

/// The colors a trigger paints: foreground labels, the Thinking slash and effort in the muted
/// foreground, and the agent icon in the agent's color (`ModelControls.tsx:2358,2739,2855`).
fn trigger_paint(state: &State, palette: Palette, layout: TriggerKind) -> TriggerPaint {
    let foreground = palette.foreground.into();
    let muted = palette.muted_foreground.into();
    match layout {
        TriggerKind::Model => TriggerPaint {
            label: foreground,
            slash: foreground,
            icon: foreground,
            muted,
        },
        TriggerKind::Thinking => TriggerPaint {
            label: muted,
            slash: muted,
            icon: foreground,
            muted,
        },
        TriggerKind::Agent => TriggerPaint {
            label: foreground,
            slash: foreground,
            icon: crate::theme::agent_color(
                &crate::views::popover::shown_agent_id(state),
                &palette,
            )
            .into(),
            muted,
        },
    }
}

fn can_send(state: &State) -> bool {
    state.connection() == &ConnectionStatus::Connected
        && state.composer().status != ComposerStatus::Sending
        && !state.composer().draft.trim().is_empty()
}

#[cfg(test)]
#[path = "composer_controls_test.rs"]
mod composer_controls_test;
