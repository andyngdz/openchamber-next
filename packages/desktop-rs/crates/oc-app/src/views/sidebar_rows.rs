//! One session row under a section or project group.

use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, Div, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{Action, SessionEntry, SidebarRow, State};

use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Reveal, Role, control, mark};
use crate::views::Scene;

pub(crate) const ARCHIVE: &str = "Archive";
pub(crate) const TRACK_WORK: &str = "Track as in work";
pub(crate) const MARK_DONE: &str = "Mark done";
const DRAGGABLE: &str = "draggable";
const UNTITLED: &str = "Untitled session";
/// Height of a session, project or Show more row.
pub(super) const ROW_HEIGHT: f32 = 30.;
/// Radius of a row's hover and active fill.
pub(super) const ROW_RADIUS: f32 = 7.;
/// Left padding of a row directly under a section.
pub(super) const TOP_INDENT: f32 = 8.;
/// Left padding of a session nested under a project.
pub(super) const NESTED_INDENT: f32 = 30.;
/// Side of a hover action button.
pub(super) const ACTION_SIZE: f32 = 24.;
/// Space between a row's parts and its right padding.
const ROW_GAP: f32 = 8.;
const MARK_SIZE: f32 = 6.;
const SPINNER_SIZE: f32 = 12.;
const ROW_TEXT_SIZE: f32 = 13.;

/// What a session row says about its session by colour, besides the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowState {
    /// The agent is working: the info spinner.
    Running,
    /// The agent waits for the user: the warning dot.
    Waiting,
    /// Nothing to say.
    Idle,
}

/// A pending request outranks running, since it is the one the user has to act on.
fn row_state(state: &State, row: &SidebarRow) -> RowState {
    let waiting = state
        .sessions()
        .get(&row.id)
        .is_some_and(SessionEntry::is_waiting);
    if waiting {
        RowState::Waiting
    } else if row.busy {
        RowState::Running
    } else {
        RowState::Idle
    }
}

/// One session row: a draggable container around the title button and its hover actions.
/// `indent` is the left padding, so a nested session sits under its project.
pub(super) fn session_row(
    scene: Scene<'_>,
    row: &SidebarRow,
    actions: &[&'static str],
    indent: f32,
) -> AnyElement {
    let title = row.title.clone().unwrap_or_else(|| UNTITLED.to_owned());
    let container =
        ControlSpec::named(Area::Sidebar, Role::Button, title.clone()).described(DRAGGABLE);
    let button = ControlSpec::named(Area::Sidebar, Role::Button, title.clone());
    let hover_group: SharedString = container.key.clone();
    let open = is_open(scene.state, row);
    let state = row_state(scene.state, row);
    let tone = if open || row.busy {
        TitleTone::Emphasized
    } else {
        TitleTone::Resting
    };
    let label = row_label(&button, row, tone, scene);
    let overlay = action_overlay(scene, actions, &title, &hover_group);
    let palette = scene.palette;
    let body = div()
        .id(container.key.clone())
        .group(hover_group.clone())
        .relative()
        .w_full()
        .h(px(ROW_HEIGHT))
        .pl(px(indent))
        .pr(px(TOP_INDENT))
        .flex()
        .items_center()
        .gap(px(ROW_GAP))
        .rounded(px(ROW_RADIUS))
        .when(open, |style| style.bg(Hsla::from(palette.active)))
        .when(!open, |style| {
            style.hover(|hovered| hovered.bg(Hsla::from(palette.hover)))
        })
        .on_click(scene.on_click_dispatch(Action::OpenSession(row.id.clone())))
        .child(
            control(scene.registry, button.clone(), label)
                .flex_1()
                .min_w_0(),
        )
        .children(state_mark(scene, state, &button, &hover_group))
        .child(overlay);
    control(scene.registry, container, body)
        .w_full()
        .into_any_element()
}

/// The hover actions, laid over the row's end; the state mark steps aside while they show.
fn action_overlay(
    scene: Scene<'_>,
    actions: &[&'static str],
    title: &str,
    hover_group: &SharedString,
) -> Div {
    let buttons = actions.iter().map(|action| {
        let spec = ControlSpec::named(Area::Sidebar, Role::Button, *action)
            .under(&[title])
            .revealed(Reveal::OnHover(hover_group.clone()));
        let icon = action_icon(action);
        control(
            scene.registry,
            spec.clone(),
            crate::views::plain_icon_button(&spec, icon)
                .w(px(ACTION_SIZE))
                .h(px(ACTION_SIZE)),
        )
    });
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .right(px(4.))
        .flex()
        .items_center()
        .children(buttons)
}

/// The colour mark at the row's end: the info spinner while running, the warning dot while
/// waiting. The session list has no unread flag yet, so no done-unread dot is drawn.
fn state_mark(
    scene: Scene<'_>,
    state: RowState,
    button: &ControlSpec,
    hover_group: &SharedString,
) -> Option<Div> {
    let palette = scene.palette;
    let slot = div()
        .flex_none()
        .flex()
        .items_center()
        .group_hover(hover_group.clone(), |style| style.opacity(0.));
    match state {
        RowState::Idle => None,
        RowState::Running => {
            let color = Hsla::from(palette.status_info);
            // Unrecorded: a 2px ring never paints its full color, so the frame check rejects it.
            Some(
                slot.child(
                    Icon::new(RemixIcon::Loader4)
                        .with_size(px(SPINNER_SIZE))
                        .text_color(color),
                ),
            )
        }
        RowState::Waiting => {
            let color = Hsla::from(palette.status_warning);
            let dot = mark(
                scene.registry,
                &button.key,
                color,
                px(MARK_SIZE),
                px(MARK_SIZE),
            )
            .rounded_full();
            Some(slot.child(dot))
        }
    }
}

/// The icon React draws on a row's hover action.
fn action_icon(action: &str) -> RemixIcon {
    if action == TRACK_WORK {
        RemixIcon::Eye
    } else if action == MARK_DONE {
        RemixIcon::Check
    } else {
        RemixIcon::Archive
    }
}

/// Which of the two title colors a row draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitleTone {
    /// `text-foreground`: the open or busy row. React also emphasizes a row with a pending
    /// request; unit 18 adds that.
    Emphasized,
    /// `oc-title-rest`: every other row (`index.css:217-219`).
    Resting,
}

/// The title in its tone: `text-foreground` for the open or busy row and
/// `oc-title-rest` for the rest (`SessionNodeItem.tsx:1680-1687`, `index.css:217-219`).
fn row_label(
    button: &ControlSpec,
    row: &SidebarRow,
    tone: TitleTone,
    scene: Scene<'_>,
) -> AnyElement {
    let title = row.title.clone().unwrap_or_else(|| UNTITLED.to_owned());
    let palette = scene.palette;
    let color = match tone {
        TitleTone::Emphasized => palette.foreground.into(),
        TitleTone::Resting => crate::theme::mix_oklab(
            palette.foreground.into(),
            palette.muted_foreground.into(),
            0.5,
        ),
    };
    let registry = scene.registry;
    let slot = registry.record_text(
        &button.key,
        crate::layout::draw::TextDraw::new(title.clone(), color),
    );
    let label = div()
        .flex_1()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_color(color)
        .text_size(px(ROW_TEXT_SIZE))
        .truncate()
        .child(title);
    crate::layout::placed(registry, slot, label)
        .id(button.key.clone())
        .into_any_element()
}

fn is_open(state: &State, row: &SidebarRow) -> bool {
    state.open().is_some_and(|open| open.id == row.id)
}

#[cfg(test)]
#[path = "sidebar_rows_test.rs"]
mod sidebar_rows_test;
