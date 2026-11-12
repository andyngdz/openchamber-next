//! The open session's messages: user messages on the right, assistant text as plain text, tool
//! calls under an Activity row, and a meta row with hover actions under each finished turn.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{Action, LoadStatus, Region, SessionStatus};

use super::composer::WIDTH;
use super::tool_rows::{ToolSpecs, activity, tool_row_specs};
use super::turns::{Block, Turn, TurnMeta, turns};
use super::{Scene, icon_button, retry_button};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Reveal, Role, control};

/// The message scroller. React gives it no name, and its text is the whole transcript, so the
/// reference names it by its role.
const SCROLLER: &str = "generic";
const SCROLL_TO_BOTTOM: &str = "Scroll to bottom";
/// Hover actions of a finished turn's meta row.
const META_ACTIONS: [(&str, RemixIcon); 4] = [
    ("Copy message text", RemixIcon::FileCopy),
    ("Continue from this answer", RemixIcon::ChatForward),
    (
        "Pin into context (survives compaction)",
        RemixIcon::Pushpin2,
    ),
    ("More actions", RemixIcon::More),
];

/// The scrolling message list and its Scroll to bottom button.
pub(crate) fn messages(scene: Scene<'_>, scroll: &ScrollHandle, cx: &App) -> Div {
    let Some(open) = scene.state.open() else {
        return div().flex_1();
    };
    let busy = scene.state.open_status() == SessionStatus::Busy;
    let all = turns(&open.messages);
    let count = all.len();
    let mut tool_specs = tool_row_specs(&all).into_iter();
    let column = div()
        .w(px(WIDTH))
        .mx_auto()
        .py(px(16.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .children(all.iter().enumerate().map(|(index, turn)| {
            // The last turn of a busy session is still running: no meta row yet.
            let meta = turn.meta().filter(|_| !(busy && index + 1 == count));
            turn_view(scene, index, turn, meta, &mut tool_specs, cx)
        }))
        .children(load_line(scene, &open.load, cx));
    let scroller_spec = ControlSpec::named(Area::ChatColumn, Role::Generic, SCROLLER);
    let scroller = div()
        .id(scroller_spec.key.clone())
        .size_full()
        .overflow_y_scroll()
        .track_scroll(scroll)
        .child(column);
    div()
        .flex_1()
        .min_h_0()
        .relative()
        .child(control(scene.registry, scroller_spec, scroller).size_full())
        .child(scroll_to_bottom(scene, scroll))
}

/// Shown once the list is scrolled up from the bottom.
fn scroll_to_bottom(scene: Scene<'_>, scroll: &ScrollHandle) -> Div {
    let at_bottom = scroll.offset().y <= -scroll.max_offset().y + px(1.);
    let reveal = if at_bottom {
        Reveal::Hidden
    } else {
        Reveal::Always
    };
    let spec =
        ControlSpec::named(Area::ChatColumn, Role::Button, SCROLL_TO_BOTTOM).revealed(reveal);
    let handle = scroll.clone();
    let button =
        icon_button(&spec, RemixIcon::ArrowDown).on_click(move |_, _, _| handle.scroll_to_bottom());
    div()
        .absolute()
        .bottom(px(8.))
        .w_full()
        .flex()
        .justify_center()
        .child(control(scene.registry, spec, button))
}

fn turn_view(
    scene: Scene<'_>,
    index: usize,
    turn: &Turn<'_>,
    meta: Option<TurnMeta>,
    tool_specs: &mut ToolSpecs,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .children(turn.user_text.map(|text| user_bubble(text, cx)))
        .children(
            turn.blocks()
                .into_iter()
                .map(|block| block_view(scene, block, tool_specs, cx)),
        )
        .children(meta.map(|meta| meta_row(scene, index, &meta, cx)))
        .into_any_element()
}

fn user_bubble(text: &str, cx: &App) -> Div {
    div().flex().justify_end().child(
        div()
            .max_w(px(560.))
            .px(px(12.))
            .py(px(8.))
            .rounded(px(12.))
            .bg(cx.theme().secondary)
            .text_sm()
            .child(text.to_owned()),
    )
}

fn block_view(
    scene: Scene<'_>,
    block: Block<'_>,
    tool_specs: &mut ToolSpecs,
    cx: &App,
) -> AnyElement {
    match block {
        Block::Text(text) => div().text_sm().child(text.to_owned()).into_any_element(),
        Block::Activity(tools) => activity(scene, tools, tool_specs, cx).into_any_element(),
    }
}

/// Model, agent and duration, with the turn's hover actions.
fn meta_row(scene: Scene<'_>, index: usize, meta: &TurnMeta, cx: &App) -> Div {
    let group: SharedString = format!("turn-{index}").into();
    div()
        .group(group.clone())
        .flex()
        .items_center()
        .gap(px(4.))
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(format!(
            "{} · {} · {}",
            meta.model, meta.agent, meta.duration
        ))
        .children(META_ACTIONS.map(|(name, icon)| {
            let spec = ControlSpec::named(Area::ChatColumn, Role::Button, name)
                .revealed(Reveal::OnHover(group.clone()));
            control(scene.registry, spec.clone(), icon_button(&spec, icon))
        }))
}

/// Loading, a failed load with retry, or the agent waiting for an answer.
fn load_line(scene: Scene<'_>, load: &LoadStatus, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    let line = |text: String| {
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(text)
    };
    match load {
        LoadStatus::Loading => Some(line("Loading messages".to_owned()).into_any_element()),
        LoadStatus::Failed { message } => Some(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(line(message.clone()))
                .child(
                    retry_button("messages-retry")
                        .on_click(scene.on_click_dispatch(Action::Retry(Region::Messages))),
                )
                .into_any_element(),
        ),
        LoadStatus::Ready if scene.state.open_is_waiting() => {
            Some(line("The agent is waiting for an answer".to_owned()).into_any_element())
        }
        LoadStatus::Ready => None,
    }
}

#[cfg(test)]
#[path = "messages_test.rs"]
mod messages_test;
