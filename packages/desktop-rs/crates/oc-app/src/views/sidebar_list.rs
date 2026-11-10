//! The sidebar's scrolling list: the session groups from [`sidebar_groups`], each control keyed
//! by the group, project or session it belongs to.

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{Action, State};

use super::sidebar_model::{GroupKind, GroupModel, RowModel, sidebar_groups};
use super::{Scene, icon_button, label_button, project_name};
use crate::layout::{Area, ControlSpec, Reveal, Role, control};

const NEW_WORKTREE: &str = "New worktree";
const NEW_DRAFT: &str = "New draft session";
/// The list's first control. React gives it no name and no text, so the reference names it by
/// its role.
const LIST_LEAD: &str = "button";
/// Role descriptions of rows that hold a control of their own name.
const SORTABLE: &str = "sortable";
const DRAGGABLE: &str = "draggable";

/// The groups under Chats; grows to fill the sidebar and scrolls.
pub(crate) fn session_list(scene: Scene<'_>, cx: &App) -> Div {
    let lead = ControlSpec::named(Area::Sidebar, Role::Button, LIST_LEAD);
    let list = div()
        .id("sidebar-list")
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .px(px(8.))
        .child(
            control(
                scene.registry,
                lead.clone(),
                div().id(lead.key.clone()).w_full().h(px(4.)),
            )
            .w_full(),
        )
        .children(
            sidebar_groups(scene.state, scene.work)
                .into_iter()
                .map(|group| render_group(scene, group, cx)),
        );
    div().flex_1().min_h_0().child(list)
}

fn render_group(scene: Scene<'_>, group: GroupModel, cx: &App) -> AnyElement {
    let label = group.kind.label();
    let row = ControlSpec::named(Area::Sidebar, Role::Button, label).described(SORTABLE);
    let toggle = ControlSpec::named(Area::Sidebar, Role::Button, label);
    let heading = div()
        .id(toggle.key.clone())
        .w_full()
        .pt(px(16.))
        .pb(px(4.))
        .px(px(8.))
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(label);
    let project = match group.kind {
        GroupKind::Projects => Some(project_row(scene)),
        GroupKind::InWork => None,
    };
    div()
        .flex()
        .flex_col()
        .child(
            control(
                scene.registry,
                row.clone(),
                div()
                    .id(row.key.clone())
                    .w_full()
                    .child(control(scene.registry, toggle, heading).w_full()),
            )
            .w_full(),
        )
        .children(project)
        .children(
            group
                .rows
                .into_iter()
                .map(|row| session_row(scene, row, cx)),
        )
        .into_any_element()
}

fn project_row(scene: Scene<'_>) -> AnyElement {
    let name = project_name(scene.state);
    let container =
        ControlSpec::named(Area::Sidebar, Role::Button, name.to_owned()).described(SORTABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, name.to_owned());
    let hover_group: SharedString = container.key.clone();
    let hover = |action: &'static str, icon: IconName| {
        let spec = ControlSpec::named(Area::Sidebar, Role::Button, action)
            .under(&[name])
            .revealed(Reveal::OnHover(hover_group.clone()));
        control(scene.registry, spec.clone(), icon_button(&spec, icon))
    };
    let row = div()
        .id(container.key.clone())
        .group(hover_group.clone())
        .w_full()
        .flex()
        .items_center()
        .child(
            control(
                scene.registry,
                button.clone(),
                label_button(&button, Some(IconName::Folder), name.to_owned()),
            )
            .flex_1(),
        )
        .child(hover(NEW_WORKTREE, IconName::Network))
        .child(hover(NEW_DRAFT, IconName::Plus));
    control(scene.registry, container, row)
        .w_full()
        .into_any_element()
}

fn session_row(scene: Scene<'_>, row: RowModel, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let title = row.title.as_str();
    let container =
        ControlSpec::named(Area::Sidebar, Role::Button, title.to_owned()).described(DRAGGABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, title.to_owned());
    let hover_group: SharedString = container.key.clone();
    let label = row_label(&button, &row, cx);
    let actions = row
        .actions
        .iter()
        .zip([IconName::CircleCheck, IconName::Inbox])
        .map(|(name, icon)| {
            let spec = ControlSpec::named(Area::Sidebar, Role::Button, *name)
                .under(&[title])
                .revealed(Reveal::OnHover(hover_group.clone()));
            control(scene.registry, spec.clone(), icon_button(&spec, icon))
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
        .when(is_open(scene.state, &row), |style| {
            style.bg(theme.sidebar_accent)
        })
        .hover(|style| style.bg(theme.sidebar_accent))
        .on_click(scene.on_click_dispatch(Action::OpenSession(row.id.clone())))
        .child(control(scene.registry, button, label).flex_1().min_w_0())
        .children(actions);
    control(scene.registry, container, body)
        .w_full()
        .into_any_element()
}

/// The title, with a spinner while the session runs a turn.
fn row_label(button: &ControlSpec, row: &RowModel, cx: &App) -> AnyElement {
    div()
        .id(button.key.clone())
        .flex_1()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_sm()
        .truncate()
        .child(row.title.clone())
        .children(row.busy.then(|| {
            div()
                .text_color(cx.theme().primary)
                .child(IconName::LoaderCircle)
        }))
        .into_any_element()
}

fn is_open(state: &State, row: &RowModel) -> bool {
    state.open().is_some_and(|open| open.id == row.id)
}

#[cfg(test)]
#[path = "sidebar_list_test.rs"]
mod sidebar_list_test;
