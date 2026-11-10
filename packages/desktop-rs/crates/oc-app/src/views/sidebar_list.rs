//! The sidebar's scrolling list: the sections, groups and rows derived from
//! [`oc_state::State::sidebar_model`].

#[path = "sidebar_rows.rs"]
mod rows;

#[path = "sidebar_sections.rs"]
mod sections;

#[path = "sidebar_projects.rs"]
mod projects;

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{Action, ConnectionStatus, SidebarModel};

use super::Scene;

pub(crate) use rows::{ARCHIVE, MARK_DONE, TRACK_WORK};

/// React's list starts with a wrapper that has no name and no text.
const LIST_LEAD: &str = "button";

/// The groups under Chats; grows to fill the sidebar and scrolls.
pub(crate) fn session_list(scene: Scene<'_>, cx: &App) -> Div {
    let model = scene.state.sidebar_model();
    let list = div()
        .id("sidebar-list")
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .px(px(8.))
        .children(status_lines(&model, scene, cx))
        .children(
            model
                .sections
                .iter()
                .filter_map(|section| sections::render(section, scene, cx)),
        );
    div().flex_1().min_h_0().child(list)
}

/// What the list shows instead of rows: server down, loading, or a failed load with retry.
fn status_lines(model: &SidebarModel, scene: Scene<'_>, cx: &App) -> Vec<AnyElement> {
    let theme = cx.theme();
    let text = |body: &str| {
        div()
            .px(px(8.))
            .py(px(4.))
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(body.to_owned())
    };
    if matches!(
        scene.state.connection(),
        ConnectionStatus::Failed(_) | ConnectionStatus::ServerExited(_)
    ) {
        return vec![text("Server not running").into_any_element()];
    }
    if let Some(error) = &model.error {
        return vec![
            div()
                .flex()
                .items_center()
                .child(text(error))
                .child(retry(scene, "sidebar-retry"))
                .into_any_element(),
        ];
    }
    if model.loading {
        return vec![text("Loading sessions").into_any_element()];
    }
    if model.incomplete {
        return vec![
            div()
                .flex()
                .items_center()
                .child(text("Session list incomplete"))
                .child(retry(scene, "session-list-retry"))
                .into_any_element(),
        ];
    }
    Vec::new()
}

fn retry(scene: Scene<'_>, id: &'static str) -> AnyElement {
    super::retry_button(id)
        .on_click(scene.on_click_dispatch(Action::Retry(oc_state::Region::Sidebar)))
        .into_any_element()
}

#[cfg(test)]
#[path = "sidebar_list_test.rs"]
mod sidebar_list_test;
