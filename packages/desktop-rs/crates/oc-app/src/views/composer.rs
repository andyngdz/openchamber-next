//! The composer: the text area and its footer. The model, Thinking and agent triggers open
//! their pickers; Send sends.

#[path = "composer_controls.rs"]
mod controls;
#[path = "composer_slots.rs"]
mod slots;

use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::{App, Div, Entity, InteractiveElement, ParentElement, Styled, Window, div, px};

use super::Scene;
use crate::layout::{Area, ControlSpec, Role, control, region};
use crate::theme::{Tokens, mix_oklab};

use controls::{left_group, right_group};

/// The text area's accessible name: its placeholder.
pub(crate) const PLACEHOLDER: &str =
    "Plan, build, or ask. @ to mention, $ for skills, / for commands";
/// The composer box size: 720.5 x 100, the Electron composer's box.
pub(crate) const WIDTH: f32 = 720.5;
const HEIGHT: f32 = 100.;
/// The mockup's `padding: 10px 10px 6px 14px`.
const PAD_TOP: f32 = 10.;
const PAD_RIGHT: f32 = 10.;
const PAD_BOTTOM: f32 = 6.;
const PAD_LEFT: f32 = 14.;
/// The footer row never wraps and stays 32 high.
const FOOTER_HEIGHT: f32 = 32.;
/// The composer's radius.
const RADIUS: f32 = 14.;
/// The composer fill: the foreground at 4% over the background.
const FILL_MIX: f32 = 0.04;
/// The text area above the footer: 100 - 10 top - 6 bottom - 32 footer.
const TEXT_HEIGHT: f32 = 52.;
const TEXT_SIZE: f32 = 14.;

/// The composer box, `720.5 x 100`; its caller places it.
pub(crate) fn composer(
    scene: Scene<'_>,
    draft: &Entity<TextareaState>,
    window: &mut Window,
    cx: &App,
) -> Div {
    let tokens = Tokens::of(cx);
    let registry = scene.registry;
    let text_spec = ControlSpec::named(Area::Composer, Role::Textbox, PLACEHOLDER);
    let text = div()
        .id(text_spec.key.clone())
        .w_full()
        .h(px(TEXT_HEIGHT))
        .text_size(px(TEXT_SIZE))
        .child(Textarea::new(draft).appearance(false));
    let tools = slots::toolbar(scene.state);
    let fill = mix_oklab(
        scene.palette.foreground.into(),
        scene.palette.background.into(),
        FILL_MIX,
    );
    // The far drop shadow of `--sh-raised` reaches 12px past the box and tints the
    // Choose project and starter chip text above and below it, so keep the edge and
    // the contact shadow only.
    let mut shadows = tokens.raised_shadows();
    shadows.pop();
    let frame = div()
        .size_full()
        .flex()
        .flex_col()
        .bg(fill)
        .rounded(px(RADIUS))
        .shadow(shadows)
        .pt(px(PAD_TOP))
        .pr(px(PAD_RIGHT))
        .pb(px(PAD_BOTTOM))
        .pl(px(PAD_LEFT))
        .child(control(registry, text_spec, text).w_full())
        .child(
            div()
                .h(px(FOOTER_HEIGHT))
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(left_group(scene, &tools[..3], registry))
                .child(right_group(scene, &tools[3..], registry, window, cx)),
        );
    let body = div().w(px(WIDTH)).h(px(HEIGHT)).flex_none().child(frame);
    region(registry, Area::Composer, body)
}

#[cfg(test)]
#[path = "composer_test.rs"]
mod composer_test;
