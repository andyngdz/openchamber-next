//! The Projects section and one project group.

use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div,
};
use oc_state::{MoreKey, ProjectGroup, SidebarAction};

use super::rows::session_row;
use super::sections::{SORTABLE, header_row, show_more};
use super::{ARCHIVE, TRACK_WORK};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Reveal, Role, control};
use crate::views::{Intent, Scene, label_button};
use gpui_kit::Hsla;

const NEW_WORKTREE: &str = "New worktree";
const NEW_DRAFT: &str = "New draft session";

/// `PROJECT_ICONS` (`packages/ui/src/lib/projectMeta.ts:9-55`): a stored key and the Remix
/// symbol React draws for it.
const PROJECT_ICONS: &[(&str, RemixIcon)] = &[
    ("code", RemixIcon::CodeBox),
    ("terminal", RemixIcon::TerminalBox),
    ("rocket", RemixIcon::Rocket),
    ("flask", RemixIcon::Flask),
    ("gamepad", RemixIcon::Gamepad),
    ("briefcase", RemixIcon::Briefcase),
    ("home", RemixIcon::Home),
    ("globe", RemixIcon::Global),
    ("leaf", RemixIcon::Leaf),
    ("shield", RemixIcon::Shield),
    ("palette", RemixIcon::Palette),
    ("server", RemixIcon::Server),
    ("phone", RemixIcon::Smartphone),
    ("database", RemixIcon::Database2),
    ("lightbulb", RemixIcon::Lightbulb),
    ("music", RemixIcon::Music),
    ("camera", RemixIcon::Camera),
    ("book", RemixIcon::BookOpen),
    ("heart", RemixIcon::Heart),
    ("cpu", RemixIcon::Cpu),
    ("cloud", RemixIcon::Cloud),
    ("store", RemixIcon::Store2),
    ("school", RemixIcon::GraduationCap),
    ("chat", RemixIcon::Chat3),
    ("mail", RemixIcon::Mail),
    ("map", RemixIcon::MapPin2),
    ("robot", RemixIcon::Robot2),
    ("ai", RemixIcon::BrainAi3),
    ("plugin", RemixIcon::Puzzle2),
    ("tools", RemixIcon::Tools),
    ("chart", RemixIcon::BarChartBox),
    ("writing", RemixIcon::PenNib),
    ("image", RemixIcon::Image2),
    ("film", RemixIcon::Film),
    ("trophy", RemixIcon::Trophy),
    ("travel", RemixIcon::Plane),
    ("coffee", RemixIcon::Cup),
    ("bug", RemixIcon::Bug),
    ("apple", RemixIcon::Apple),
    ("android", RemixIcon::Android),
    ("windows", RemixIcon::Windows),
    ("github", RemixIcon::Github),
    ("stack", RemixIcon::Stack),
    ("finance", RemixIcon::Bank),
    ("team", RemixIcon::Team),
    ("fire", RemixIcon::Fire),
    ("planet", RemixIcon::Planet),
];

fn icon_for(group: &ProjectGroup) -> RemixIcon {
    group
        .project
        .icon
        .as_deref()
        .and_then(|key| {
            PROJECT_ICONS
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, icon)| *icon)
        })
        .unwrap_or(RemixIcon::Folder)
}

fn icon_color(group: &ProjectGroup) -> Hsla {
    group
        .project
        .color
        .as_deref()
        .and_then(crate::theme::stored)
        .unwrap_or_else(|| crate::theme::palette().muted_foreground.into())
}

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
            .child(project_title(scene, group, name, &button))
            .child(project_hover(
                scene,
                name,
                hover_group.clone(),
                NEW_WORKTREE,
                RemixIcon::Worktree,
            ))
            .child(project_hover(
                scene,
                name,
                hover_group,
                NEW_DRAFT,
                RemixIcon::Add,
            ))
            .on_click(collapse),
    )
    .w_full()
}

/// The project's own button: its icon in the project's color, then its name.
fn project_title(scene: Scene<'_>, group: &ProjectGroup, name: &str, button: &ControlSpec) -> Div {
    control(
        scene.registry,
        button.clone(),
        label_button(
            scene.registry,
            button,
            Some((icon_for(group), icon_color(group))),
            name.to_owned(),
            crate::theme::palette().foreground.into(),
        ),
    )
    .flex_1()
}

/// One action revealed while the project is hovered.
fn project_hover(
    scene: Scene<'_>,
    name: &str,
    hover_group: SharedString,
    action: &'static str,
    icon: RemixIcon,
) -> AnyElement {
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, action)
        .under(&[name])
        .revealed(Reveal::OnHover(hover_group));
    control(
        scene.registry,
        spec.clone(),
        crate::views::plain_icon_button(&spec, icon),
    )
    .into_any_element()
}

#[cfg(test)]
#[path = "sidebar_projects_test.rs"]
mod sidebar_projects_test;
