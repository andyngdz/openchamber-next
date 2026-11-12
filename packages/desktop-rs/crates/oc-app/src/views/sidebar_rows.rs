//! One session row under a section or project group.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{Action, SidebarRow, State};

use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Reveal, Role, control};
use crate::views::Scene;

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
    let open = is_open(scene.state, row);
    let tone = if open || row.busy {
        TitleTone::Emphasized
    } else {
        TitleTone::Resting
    };
    let label = row_label(&button, row, tone, scene.registry);
    let action_buttons = actions.iter().map(|action| {
        let spec = ControlSpec::named(Area::Sidebar, Role::Button, *action)
            .under(&[title.as_str()])
            .revealed(Reveal::OnHover(hover_group.clone()));
        let icon = action_icon(action);
        control(
            scene.registry,
            spec.clone(),
            crate::views::plain_icon_button(&spec, icon),
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
        .when(open, |style| style.bg(theme.sidebar_accent))
        .hover(|style| style.bg(theme.sidebar_accent))
        .on_click(scene.on_click_dispatch(Action::OpenSession(row.id.clone())))
        .child(control(scene.registry, button, label).flex_1().min_w_0())
        .children(action_buttons);
    control(scene.registry, container, body)
        .w_full()
        .into_any_element()
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
    registry: &crate::layout::LayoutRegistry,
) -> AnyElement {
    let title = row.title.clone().unwrap_or_else(|| UNTITLED.to_owned());
    let palette = crate::theme::palette();
    let color = match tone {
        TitleTone::Emphasized => palette.foreground.into(),
        TitleTone::Resting => crate::theme::mix_oklab(
            palette.foreground.into(),
            palette.muted_foreground.into(),
            0.5,
        ),
    };
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
        .text_sm()
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
