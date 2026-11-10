//! The composer: the text area and its toolbar. Only the text area and send are live; the
//! other toolbar controls are inert.

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::{ActiveTheme, Disableable};
use gpui_kit::{
    AnyElement, App, Div, Entity, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use oc_state::SessionStatus;

use super::{Scene, icon_button, label_button};
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control, region};
use crate::theme::{DARK, card};

/// The text area's accessible name: its placeholder.
pub(crate) const PLACEHOLDER: &str =
    "Plan, build, or ask. @ to mention, $ for skills, / for commands";
/// The composer box size the reference measures (720.5 x 100).
pub(crate) const WIDTH: f32 = 720.5;
const HEIGHT: f32 = 100.;
/// The model button's reference key. The spike reads no model list, so this names the
/// default model of the reference run; a model picker replaces it.
const MODEL_KEY: &str = "perf logo Fixture agent, 20 tool calls then 300...";
/// The agent button: its name and its label.
const AGENT: &str = "Build";

/// One toolbar slot, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Icon(&'static str, IconName),
    Spacer,
    Labeled(&'static str, IconName, &'static str),
    Send,
    Stop,
}

fn toolbar(status: SessionStatus) -> Vec<Tool> {
    let last = match status {
        SessionStatus::Busy => Tool::Stop,
        SessionStatus::Idle | SessionStatus::Error => Tool::Send,
    };
    vec![
        Tool::Icon("Add attachment", IconName::Plus),
        Tool::Icon("Permissions: ask every time", IconName::CircleCheck),
        Tool::Icon("Start a goal with the next message", IconName::Star),
        Tool::Spacer,
        Tool::Labeled(MODEL_KEY, IconName::Bot, "Default model"),
        Tool::Labeled("Thinking: Default", IconName::Cpu, "default"),
        Tool::Labeled(AGENT, IconName::CircleUser, AGENT),
        Tool::Icon("Start dictation", IconName::Mic),
        last,
    ]
}

fn tool_name(tool: Tool) -> Option<&'static str> {
    match tool {
        Tool::Icon(name, _) | Tool::Labeled(name, _, _) => Some(name),
        Tool::Send => Some("Send message"),
        Tool::Stop => Some("Stop generating"),
        Tool::Spacer => None,
    }
}

/// The composer box, `720.5 x 100`; its caller places it.
pub(crate) fn composer(scene: Scene<'_>, draft: &Entity<TextareaState>, cx: &App) -> Div {
    let theme = cx.theme();
    let registry = scene.registry;
    let text_spec = ControlSpec::named(Area::Composer, Role::Textbox, PLACEHOLDER);
    let text = div()
        .id(text_spec.key.clone())
        .w_full()
        .h(px(56.))
        .child(Textarea::new(draft).appearance(false));
    let tools = toolbar(scene.state.open_status())
        .into_iter()
        .map(|tool| render_tool(tool, scene, registry));
    let frame = div()
        .size_full()
        .flex()
        .flex_col()
        .rounded(px(12.))
        .border_1()
        .border_color(theme.border)
        .bg(card(DARK))
        .px(px(4.))
        .pt(px(4.))
        .child(control(registry, text_spec, text).w_full())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .px(px(4.))
                .children(tools),
        );
    let body = div().w(px(WIDTH)).h(px(HEIGHT)).flex_none().child(frame);
    region(registry, Area::Composer, body)
}

fn render_tool(tool: Tool, scene: Scene<'_>, registry: &LayoutRegistry) -> AnyElement {
    let Some(name) = tool_name(tool) else {
        return div().flex_1().into_any_element();
    };
    let spec = ControlSpec::named(Area::Composer, Role::Button, name);
    let button = match tool {
        Tool::Icon(_, icon) => icon_button(&spec, icon),
        Tool::Labeled(_, icon, label) => label_button(&spec, Some(icon), label),
        Tool::Send => icon_button(&spec, IconName::ArrowUp).disabled(!scene.state.can_send()),
        Tool::Stop => icon_button(&spec, IconName::Square),
        Tool::Spacer => return div().into_any_element(),
    };
    control(registry, spec, button).into_any_element()
}

#[cfg(test)]
#[path = "composer_test.rs"]
mod composer_test;
