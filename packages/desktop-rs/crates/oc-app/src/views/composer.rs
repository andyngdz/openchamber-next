//! The composer: the text area and its footer. The model, Thinking and agent triggers open
//! their pickers; Send sends.

#[path = "composer_controls.rs"]
mod controls;
#[path = "composer_slots.rs"]
mod slots;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::{App, Div, Entity, InteractiveElement, ParentElement, Styled, Window, div, px};

use super::Scene;
use crate::layout::{Area, ControlSpec, Role, control, region};
use crate::theme::{DARK, card};

use controls::{left_group, right_group};

/// The text area's accessible name: its placeholder.
pub(crate) const PLACEHOLDER: &str =
    "Plan, build, or ask. @ to mention, $ for skills, / for commands";
/// The composer box size the reference measures (720.5 x 100).
pub(crate) const WIDTH: f32 = 720.5;
const HEIGHT: f32 = 100.;
/// The footer's `px-2.5 py-1.5` (`ChatInput.tsx:4957`).
const FOOTER_PADDING: f32 = 10.;
const FOOTER_VERTICAL: f32 = 6.;
const FOOTER_HEIGHT: f32 = 44.;
/// The text area above the footer: 100 - 4 top padding - 44 footer.
const TEXT_HEIGHT: f32 = 52.;

/// The composer box, `720.5 x 100`; its caller places it.
pub(crate) fn composer(
    scene: Scene<'_>,
    draft: &Entity<TextareaState>,
    window: &mut Window,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let registry = scene.registry;
    let text_spec = ControlSpec::named(Area::Composer, Role::Textbox, PLACEHOLDER);
    let text = div()
        .id(text_spec.key.clone())
        .w_full()
        .h(px(TEXT_HEIGHT))
        .px(px(4.))
        .child(Textarea::new(draft).appearance(false));
    let tools = slots::toolbar(scene.state);
    let frame = div()
        .size_full()
        .flex()
        .flex_col()
        .rounded(px(12.))
        .border_1()
        .border_color(theme.border)
        .bg(card(DARK))
        .pt(px(4.))
        .child(control(registry, text_spec, text).w_full())
        .child(
            div()
                .h(px(FOOTER_HEIGHT))
                .flex()
                .items_center()
                .justify_between()
                .px(px(FOOTER_PADDING))
                .py(px(FOOTER_VERTICAL))
                .child(left_group(&tools[..3], registry))
                .child(right_group(scene, &tools[3..], registry, window, cx)),
        );
    let body = div().w(px(WIDTH)).h(px(HEIGHT)).flex_none().child(frame);
    region(registry, Area::Composer, body)
}

#[cfg(test)]
#[path = "composer_test.rs"]
mod composer_test;
