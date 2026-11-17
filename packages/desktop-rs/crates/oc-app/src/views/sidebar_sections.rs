//! One sidebar section: Chats, Pinned, In work or the project groups.

use gpui_kit::{
    AnyElement, Div, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{MoreKey, SidebarAction, SidebarRow, SidebarSectionModel};

use super::LIST_LEAD;
use super::rows::{ACTION_SIZE, ROW_HEIGHT, ROW_RADIUS, TOP_INDENT, session_row};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Reveal, Role, control};
use crate::views::{Intent, Scene, draw_text};

pub(crate) const CHATS: &str = "Chats";
pub(crate) const PINNED: &str = "Pinned";
pub(crate) const IN_WORK: &str = "In work";
pub(crate) const PROJECTS: &str = "Projects";
const SHOW_MORE: &str = "Show more";
const CHATS_GROUP: &str = "sidebar-chats";
pub(super) const SORTABLE: &str = "sortable";
/// Space above a section caption, which stands in for the rule between sections.
const CAPTION_GAP: f32 = 18.;
/// Height of the caption line, tall enough for the Chats add button.
const CAPTION_HEIGHT: f32 = 24.;
const CAPTION_TEXT_SIZE: f32 = 13.;

/// One section, or `None` when React hides an empty one.
pub(super) fn render(section: &SidebarSectionModel, scene: Scene<'_>) -> Option<AnyElement> {
    match section {
        SidebarSectionModel::Chats { rows, shown } if !rows.is_empty() => {
            Some(chats(scene, rows, *shown))
        }
        SidebarSectionModel::Pinned { rows, shown } if !rows.is_empty() => Some(simple(
            scene,
            PINNED,
            MoreKey::Pinned,
            rows,
            *shown,
            super::TRACK_WORK,
        )),
        SidebarSectionModel::InWork { rows, shown } if !rows.is_empty() => Some(simple(
            scene,
            IN_WORK,
            MoreKey::Work,
            rows,
            *shown,
            super::MARK_DONE,
        )),
        SidebarSectionModel::Projects { groups } => Some(super::projects::projects(scene, groups)),
        _ => None,
    }
}

fn chats(scene: Scene<'_>, rows: &[SidebarRow], shown: usize) -> AnyElement {
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
            div().id(lead.key.clone()).w_full().h(px(2.)),
        ))
        .children(
            rows.iter()
                .take(shown)
                .map(|row| session_row(scene, row, &[super::ARCHIVE], TOP_INDENT)),
        )
        .children(show_more(
            scene,
            rows,
            shown,
            MoreKey::Chats,
            CHATS,
            TOP_INDENT,
        ))
        .into_any_element()
}

fn chats_header(scene: Scene<'_>, header: &ControlSpec, add: &ControlSpec) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .pt(px(CAPTION_GAP))
        .px(px(TOP_INDENT))
        .child(control(
            scene.registry,
            header.clone(),
            caption(scene, header, CHATS),
        ))
        .child(control(
            scene.registry,
            add.clone(),
            crate::views::plain_icon_button(add, RemixIcon::Add)
                .w(px(ACTION_SIZE))
                .h(px(ACTION_SIZE))
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
                .map(|row| session_row(scene, row, &[action, super::ARCHIVE], TOP_INDENT)),
        )
        .children(show_more(scene, rows, shown, more, label, TOP_INDENT))
        .into_any_element()
}

/// The `label (sortable)` container around the section's caption.
pub(super) fn header_row(
    scene: Scene<'_>,
    container: &ControlSpec,
    button: &ControlSpec,
    label: &'static str,
) -> Div {
    control(
        scene.registry,
        container.clone(),
        div()
            .id(container.key.clone())
            .w_full()
            .pt(px(CAPTION_GAP))
            .px(px(TOP_INDENT))
            .child(control(
                scene.registry,
                button.clone(),
                caption(scene, button, label),
            )),
    )
    .w_full()
}

/// A section's name as a bare muted word: no fill, no rule.
fn caption(scene: Scene<'_>, button: &ControlSpec, label: &'static str) -> Div {
    let muted = Hsla::from(scene.palette.muted_foreground);
    draw_text(scene.registry, button, label, muted);
    div()
        .h(px(CAPTION_HEIGHT))
        .flex()
        .items_center()
        .text_size(px(CAPTION_TEXT_SIZE))
        .text_color(muted)
        .child(label)
}

/// The Show more row of a section or project, when rows remain hidden.
pub(super) fn show_more(
    scene: Scene<'_>,
    rows: &[SidebarRow],
    shown: usize,
    more: MoreKey,
    ancestor: &str,
    indent: f32,
) -> Option<AnyElement> {
    if shown >= rows.len() {
        return None;
    }
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, SHOW_MORE).under(&[ancestor]);
    let muted = Hsla::from(scene.palette.muted_foreground);
    let hover = Hsla::from(scene.palette.hover);
    draw_text(scene.registry, &spec, SHOW_MORE, muted);
    let key: SharedString = spec.key.clone();
    let row = div()
        .id(key)
        .w_full()
        .h(px(ROW_HEIGHT))
        .pl(px(indent))
        .flex()
        .items_center()
        .rounded(px(ROW_RADIUS))
        .text_size(px(CAPTION_TEXT_SIZE))
        .text_color(muted)
        .hover(|style| style.bg(hover))
        .on_click(scene.on_click_intent(Intent::ApplySidebar(SidebarAction::ShowMore(more))))
        .child(SHOW_MORE);
    Some(
        control(scene.registry, spec, row)
            .w_full()
            .into_any_element(),
    )
}

#[cfg(test)]
#[path = "sidebar_sections_test.rs"]
mod sidebar_sections_test;
