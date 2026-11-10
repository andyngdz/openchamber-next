//! The composer footer's left icon group and right trigger/action group.

use gpui_kit::StatefulInteractiveElement;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable;
use gpui_kit::{
    AnyElement, App, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, px,
};
use oc_state::{ComposerStatus, ConnectionStatus, State};

use super::slots::{Tool, TriggerKind, tool_name};
use crate::layout::anchored_control;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};
use crate::views::{Intent, Scene, icon_button};

/// `h-8` (`ModelControls.tsx:677`).
const TRIGGER_HEIGHT: f32 = 32.;
/// `providerLogoSize size-[17px]` (`ModelControls.tsx:679`).
const LOGO_SIZE: f32 = 17.;
/// `controlIconSize size-4` (`ModelControls.tsx:678`).
const ICON_SIZE: f32 = 16.;
/// `gap-1.5` for the model/agent labels (`ModelControls.tsx:2358,2855`).
const GAP_MODEL: f32 = 6.;
/// `gap-1` between the Thinking slash and label (`ModelControls.tsx:2739`).
const GAP_THINKING: f32 = 4.;
const GAP_AGENT: f32 = 6.;
/// `gap-x-3` between the footer's flex children (`ComposerFooter.tsx:284-288`).
const GAP_GROUP: f32 = 12.;
/// The first footer group's `gap-x-1.5` (`ComposerFooter.tsx:264`).
const GAP_LEFT: f32 = 6.;

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
            Tool::Trigger { .. } => trigger(scene, tool, registry, window, cx),
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

fn trigger(
    scene: Scene<'_>,
    tool: &Tool,
    registry: &LayoutRegistry,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
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
    let label_width = crate::views::line_width(window, cx, label, 12.5, 500.);
    let width = trigger_width(*layout, label_width, window, cx);
    let trigger = trigger_content(*layout, label, label_width)
        .w(px(width))
        .h(px(TRIGGER_HEIGHT))
        .id(spec.key.clone())
        .on_click(scene.on_click_intent(Intent::OpenPopover(*kind)));
    anchored_control(
        registry,
        spec,
        trigger,
        scene.triggers.for_kind(*kind).clone(),
    )
    .into_any_element()
}

fn trigger_content(layout: TriggerKind, label: &str, label_width: f32) -> Div {
    match layout {
        TriggerKind::Model => div()
            .flex()
            .items_center()
            .gap(px(GAP_MODEL))
            .child(icon_box(LOGO_SIZE, IconName::Bot))
            .child(label_box(label, label_width)),
        TriggerKind::Thinking => div()
            .flex()
            .items_center()
            .gap(px(GAP_THINKING))
            .child(div().text_size(px(16.)).line_height(px(24.)).child("/"))
            .child(label_box(label, label_width)),
        TriggerKind::Agent => div()
            .flex()
            .items_center()
            .gap(px(GAP_AGENT))
            .child(icon_box(ICON_SIZE, IconName::CircleUser))
            .child(label_box(label, label_width)),
    }
}

fn trigger_width(layout: TriggerKind, label_width: f32, window: &mut Window, cx: &App) -> f32 {
    match layout {
        TriggerKind::Model => LOGO_SIZE + GAP_MODEL + label_width,
        TriggerKind::Thinking => {
            crate::views::line_width(window, cx, "/", 16., 400.) + GAP_THINKING + label_width
        }
        TriggerKind::Agent => ICON_SIZE + GAP_AGENT + label_width,
    }
}

fn label_box(label: &str, width: f32) -> Div {
    div()
        .w(px(width))
        .truncate()
        .text_size(px(12.5))
        .line_height(px(18.))
        .font_weight(FontWeight(500.))
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
