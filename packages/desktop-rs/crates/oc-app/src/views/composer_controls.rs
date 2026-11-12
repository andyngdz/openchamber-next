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

fn icon_control(name: &str, icon: RemixIcon, registry: &LayoutRegistry) -> AnyElement {
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    let foreground = crate::theme::palette().foreground.into();
    control(
        registry,
        spec.clone(),
        icon_button(registry, &spec, icon, foreground),
    )
    .into_any_element()
}

fn action_control(scene: Scene<'_>, tool: &Tool, registry: &LayoutRegistry) -> AnyElement {
    let name = tool_name(tool).unwrap_or_default();
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    let palette = crate::theme::palette();
    let button = match tool {
        // React's own `StopIcon` (`components/icons/StopIcon.tsx`), in the error color.
        Tool::Stop => {
            registry.record_icon(
                &spec.key,
                crate::layout::draw::IconDraw::Custom {
                    color: palette.destructive.into(),
                },
            );
            let danger: gpui_kit::Hsla = palette.destructive.into();
            Button::new(spec.key.clone())
                .ghost()
                .small()
                .text_color(danger)
                .icon(gpui_kit::component::Icon::new(
                    crate::icons::CustomIcon::Stop,
                ))
                .tooltip(spec.name.clone())
        }
        // `text-primary` while sending is possible, else the foreground at 30% (`opacity-30`)
        // (`ComposerActionButtons.tsx:77-82`).
        _ => {
            let enabled = can_send(scene.state);
            let color = if enabled {
                palette.primary.into()
            } else {
                crate::theme::faded(palette.foreground.into(), 0.3)
            };
            icon_button(registry, &spec, RemixIcon::SendPlane2, color)
                .disabled(!enabled)
                .on_click(scene.on_click_intent(Intent::SendPrompt))
        }
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
    let paint = trigger_paint(scene.state, *layout);
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

/// The colors a trigger paints: `text-foreground` labels, the Thinking slash in
/// `text-muted-foreground/60` and its label in `text-status-warning/75`, and the agent icon
/// in the agent's color (`ModelControls.tsx:2358,2739,2855`).
fn trigger_paint(state: &State, layout: TriggerKind) -> TriggerPaint {
    let palette = crate::theme::palette();
    let foreground = palette.foreground.into();
    match layout {
        TriggerKind::Model => TriggerPaint {
            label: foreground,
            slash: foreground,
            icon: foreground,
        },
        TriggerKind::Thinking => TriggerPaint {
            label: crate::theme::faded(palette.status_warning.into(), 0.75),
            slash: crate::theme::faded(palette.muted_foreground.into(), 0.6),
            icon: foreground,
        },
        TriggerKind::Agent => TriggerPaint {
            label: foreground,
            slash: foreground,
            icon: crate::theme::agent_color(
                &crate::views::popover::shown_agent_id(state),
                &palette,
            )
            .into(),
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
