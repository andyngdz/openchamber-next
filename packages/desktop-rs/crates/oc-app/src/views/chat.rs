//! The chat column between the sidebar and the Work status panel: the empty state, or the
//! open session's messages with the composer at the bottom.

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::TextareaState;
use gpui_kit::{App, Div, Entity, ParentElement, ScrollHandle, Styled, div, px};
use oc_state::Region;

use super::Scene;
use super::composer::composer;
use super::empty_state::empty_state;
use super::messages::messages;
use crate::layout::{Area, region};

/// Gap between the composer and the window's bottom edge.
const COMPOSER_BOTTOM: f32 = 16.;

/// The chat column; it takes the width the fixed columns leave.
pub(crate) fn chat_column(
    scene: Scene<'_>,
    draft: &Entity<TextareaState>,
    scroll: &ScrollHandle,
    cx: &App,
) -> Div {
    let column = div().flex_1().min_w_0().h_full();
    let content = match scene.state.open() {
        None => column.child(empty_state(scene, draft, cx)),
        Some(_) => column
            .flex()
            .flex_col()
            .child(messages(scene, scroll, cx))
            .children(composer_error(scene, cx))
            .child(
                div()
                    .flex()
                    .justify_center()
                    .pb(px(COMPOSER_BOTTOM))
                    .child(composer(scene, draft, cx)),
            ),
    };
    region(scene.registry, Area::ChatColumn, content)
}

/// A failed prompt's error, above the composer; the draft stays in the composer.
fn composer_error(scene: Scene<'_>, cx: &App) -> Option<Div> {
    let message = scene.state.region_error(Region::Composer)?;
    Some(
        div()
            .flex()
            .justify_center()
            .pb(px(8.))
            .text_xs()
            .text_color(cx.theme().danger)
            .child(message.to_owned()),
    )
}

#[cfg(test)]
#[path = "chat_test.rs"]
mod chat_test;
