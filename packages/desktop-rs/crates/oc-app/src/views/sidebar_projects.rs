//! The Projects section and one project group.

use gpui_kit::assets::IconName;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div,
};
use oc_state::{MoreKey, ProjectGroup, SidebarAction};

use super::rows::session_row;
use super::sections::{SORTABLE, header_row, show_more};
use super::{ARCHIVE, TRACK_WORK};
use crate::layout::{Area, ControlSpec, Reveal, Role, control};
use crate::views::{Intent, Scene, icon_button, label_button};

const NEW_WORKTREE: &str = "New worktree";
const NEW_DRAFT: &str = "New draft session";

/// The Projects heading and its groups.
pub(super) fn projects(scene: Scene<'_>, groups: &[ProjectGroup], cx: &App) -> AnyElement {
    let container = ControlSpec::named(Area::Sidebar, Role::Button, super::sections::PROJECTS)
        .described(SORTABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, super::sections::PROJECTS);
    div()
        .flex()
        .flex_col()
        .child(header_row(
            scene,
            &container,
            &button,
            super::sections::PROJECTS,
        ))
        .children(groups.iter().map(|group| project_group(scene, group, cx)))
        .into_any_element()
}

fn project_group(scene: Scene<'_>, group: &ProjectGroup, cx: &App) -> AnyElement {
    let name = group.project.label.clone();
    let header = project_header(scene, group, &name);
    let mut column = div().flex().flex_col().child(header);
    if !group.project.collapsed {
        column = column.children(
            group
                .rows
                .iter()
                .take(group.shown)
                .map(|row| session_row(scene, row, &[TRACK_WORK, ARCHIVE], cx)),
        );
        if let Some(more) = show_more(
            scene,
            &group.rows,
            group.shown,
            MoreKey::Project(group.project.id.clone()),
            &name,
        ) {
            column = column.child(more);
        }
    }
    column.into_any_element()
}

fn project_header(scene: Scene<'_>, group: &ProjectGroup, name: &str) -> Div {
    let container =
        ControlSpec::named(Area::Sidebar, Role::Button, name.to_owned()).described(SORTABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, name.to_owned());
    let hover_group: SharedString = container.key.clone();
    let hover = |action: &'static str, icon: IconName, hover_group: SharedString| {
        let spec = ControlSpec::named(Area::Sidebar, Role::Button, action)
            .under(&[name])
            .revealed(Reveal::OnHover(hover_group));
        control(scene.registry, spec.clone(), icon_button(&spec, icon))
    };
    let collapse = scene.on_click_intent(Intent::ApplySidebar(
        SidebarAction::ToggleProjectCollapsed(group.project.id.clone()),
    ));
    control(
        scene.registry,
        container.clone(),
        div()
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
            .child(hover(NEW_WORKTREE, IconName::Network, hover_group.clone()))
            .child(hover(NEW_DRAFT, IconName::Plus, hover_group))
            .on_click(collapse),
    )
    .w_full()
}

#[cfg(test)]
#[path = "sidebar_projects_test.rs"]
mod sidebar_projects_test;
