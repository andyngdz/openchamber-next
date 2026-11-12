//! One sidebar section: Chats, Pinned, In work or the project groups.

use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use oc_state::{MoreKey, SidebarAction, SidebarRow, SidebarSectionModel};

use super::LIST_LEAD;
use super::rows::session_row;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Reveal, Role, control};
use crate::views::{Intent, Scene, label_button};

pub(crate) const CHATS: &str = "Chats";
pub(crate) const PINNED: &str = "Pinned";
pub(crate) const IN_WORK: &str = "In work";
pub(crate) const PROJECTS: &str = "Projects";
const SHOW_MORE: &str = "Show more";
const CHATS_GROUP: &str = "sidebar-chats";
pub(super) const SORTABLE: &str = "sortable";

/// One section, or `None` when React hides an empty one.
pub(super) fn render(
    section: &SidebarSectionModel,
    scene: Scene<'_>,
    cx: &App,
) -> Option<AnyElement> {
    match section {
        SidebarSectionModel::Chats { rows, shown } if !rows.is_empty() => {
            Some(chats(scene, rows, *shown, cx))
        }
        SidebarSectionModel::Pinned { rows, shown } if !rows.is_empty() => Some(simple(
            scene,
            PINNED,
            MoreKey::Pinned,
            rows,
            *shown,
            super::TRACK_WORK,
            cx,
        )),
        SidebarSectionModel::InWork { rows, shown } if !rows.is_empty() => Some(simple(
            scene,
            IN_WORK,
            MoreKey::Work,
            rows,
            *shown,
            super::MARK_DONE,
            cx,
        )),
        SidebarSectionModel::Projects { groups } => {
            Some(super::projects::projects(scene, groups, cx))
        }
        _ => None,
    }
}

fn chats(scene: Scene<'_>, rows: &[SidebarRow], shown: usize, cx: &App) -> AnyElement {
    let header = ControlSpec::named(Area::Sidebar, Role::Button, CHATS);
    let add = ControlSpec::named(
        Area::Sidebar,
        Role::Button,
        crate::views::sidebar::NEW_SESSION,
    )
    .repeated(2)
    .revealed(Reveal::OnHover(CHATS_GROUP.into()));
    let lead = ControlSpec::named(Area::Sidebar, Role::Button, LIST_LEAD);
    div()
        .group(CHATS_GROUP)
        .flex()
        .flex_col()
        .child(chats_header(scene, &header, &add))
        .child(control(
            scene.registry,
            lead.clone(),
            div().id(lead.key.clone()).w_full().h(px(4.)),
        ))
        .children(
            rows.iter()
                .take(shown)
                .map(|row| session_row(scene, row, &[super::ARCHIVE], cx)),
        )
        .children(show_more(scene, rows, shown, MoreKey::Chats, CHATS))
        .into_any_element()
}

fn chats_header(scene: Scene<'_>, header: &ControlSpec, add: &ControlSpec) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(8.))
        .child(control(
            scene.registry,
            header.clone(),
            label_button(
                scene.registry,
                header,
                None,
                CHATS,
                crate::theme::palette().muted_foreground.into(),
            ),
        ))
        .child(control(
            scene.registry,
            add.clone(),
            crate::views::plain_icon_button(add, RemixIcon::Add)
                .on_click(scene.on_click_intent(Intent::NewSession)),
        ))
}

fn simple(
    scene: Scene<'_>,
    label: &'static str,
    more: MoreKey,
    rows: &[SidebarRow],
    shown: usize,
    action: &'static str,
    cx: &App,
) -> AnyElement {
    let container = ControlSpec::named(Area::Sidebar, Role::Button, label).described(SORTABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, label);
    div()
        .flex()
        .flex_col()
        .child(header_row(scene, &container, &button, label))
        .children(
            rows.iter()
                .take(shown)
                .map(|row| session_row(scene, row, &[action, super::ARCHIVE], cx)),
        )
        .children(show_more(scene, rows, shown, more, label))
        .into_any_element()
}

/// The `label (sortable)` container around the section's label button.
pub(super) fn header_row(
    scene: Scene<'_>,
    container: &ControlSpec,
    button: &ControlSpec,
    label: &'static str,
) -> Div {
    control(
        scene.registry,
        container.clone(),
        div().id(container.key.clone()).w_full().child(control(
            scene.registry,
            button.clone(),
            label_button(
                scene.registry,
                button,
                None,
                label,
                crate::theme::palette().muted_foreground.into(),
            ),
        )),
    )
    .w_full()
}

/// The Show more row of a section or project, when rows remain hidden.
pub(super) fn show_more(
    scene: Scene<'_>,
    rows: &[SidebarRow],
    shown: usize,
    more: MoreKey,
    ancestor: &str,
) -> Option<AnyElement> {
    if shown >= rows.len() {
        return None;
    }
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, SHOW_MORE).under(&[ancestor]);
    Some(
        control(
            scene.registry,
            spec.clone(),
            label_button(
                scene.registry,
                &spec,
                None,
                SHOW_MORE,
                crate::theme::palette().muted_foreground.into(),
            )
            .on_click(scene.on_click_intent(Intent::ApplySidebar(SidebarAction::ShowMore(more)))),
        )
        .w_full()
        .into_any_element(),
    )
}

#[cfg(test)]
#[path = "sidebar_sections_test.rs"]
mod sidebar_sections_test;
