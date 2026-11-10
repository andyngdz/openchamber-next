//! One session row under a section or project group.

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{Action, SidebarRow, State};

use crate::layout::{Area, ControlSpec, Reveal, Role, control};
use crate::views::{Scene, icon_button};

pub(crate) const ARCHIVE: &str = "Archive";
pub(crate) const TRACK_WORK: &str = "Track as in work";
pub(crate) const MARK_DONE: &str = "Mark done";
const DRAGGABLE: &str = "draggable";
const UNTITLED: &str = "Untitled session";

/// One session row: a draggable container around the title button and its hover actions.
pub(super) fn session_row(
    scene: Scene<'_>,
    row: &SidebarRow,
    actions: &[&'static str],
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let title = row.title.clone().unwrap_or_else(|| UNTITLED.to_owned());
    let container =
        ControlSpec::named(Area::Sidebar, Role::Button, title.clone()).described(DRAGGABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, title.clone());
    let hover_group: SharedString = container.key.clone();
    let label = row_label(&button, row, cx);
    let icon = |index: usize| {
        if index == 0 {
            IconName::CircleCheck
        } else {
            IconName::Inbox
        }
    };
    let action_buttons = actions.iter().enumerate().map(|(index, action)| {
        let spec = ControlSpec::named(Area::Sidebar, Role::Button, *action)
            .under(&[title.as_str()])
            .revealed(Reveal::OnHover(hover_group.clone()));
        control(
            scene.registry,
            spec.clone(),
            icon_button(&spec, icon(index)),
        )
    });
    let body = div()
        .id(container.key.clone())
        .group(hover_group.clone())
        .w_full()
        .h(px(32.))
        .pl(px(20.))
        .pr(px(4.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .when(is_open(scene.state, row), |style| {
            style.bg(theme.sidebar_accent)
        })
        .hover(|style| style.bg(theme.sidebar_accent))
        .on_click(scene.on_click_dispatch(Action::OpenSession(row.id.clone())))
        .child(control(scene.registry, button, label).flex_1().min_w_0())
        .children(action_buttons);
    control(scene.registry, container, body)
        .w_full()
        .into_any_element()
}

/// The title, with a spinner while the session runs a turn.
fn row_label(button: &ControlSpec, row: &SidebarRow, cx: &App) -> AnyElement {
    let title = row.title.clone().unwrap_or_else(|| UNTITLED.to_owned());
    div()
        .id(button.key.clone())
        .flex_1()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_sm()
        .truncate()
        .child(title)
        .children(row.busy.then(|| {
            div()
                .text_color(cx.theme().primary)
                .child(IconName::LoaderCircle)
        }))
        .into_any_element()
}

fn is_open(state: &State, row: &SidebarRow) -> bool {
    state.open().is_some_and(|open| open.id == row.id)
}

#[cfg(test)]
#[path = "sidebar_rows_test.rs"]
mod sidebar_rows_test;
