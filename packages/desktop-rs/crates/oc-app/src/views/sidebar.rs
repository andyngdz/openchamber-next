//! The 280 px sidebar: title-bar controls, toolbar, Chats, the session groups and the footer.
//! Live: session rows open their session. Everything else is inert.

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use oc_state::{ConnectionStatus, LoadStatus, Region};

use super::sidebar_list::session_list;
use super::{Scene, icon_button, label_button, retry_button};
use crate::layout::{Area, ControlSpec, Reveal, Role, control, region};

/// Title-bar controls, left to right.
const TITLE_BAR: [(&str, IconName); 2] = [
    ("Open OpenChamber menu", IconName::Menu),
    ("Open sessions", IconName::PanelLeft),
];
const NEW_SESSION: &str = "New session";
const RESIZE: &str = "Resize left panel";
const TOOLBAR: [(&str, IconName); 4] = [
    ("Search sessions", IconName::Search),
    ("Add project", IconName::FolderOpen),
    ("Display mode", IconName::SortAscending),
    ("More", IconName::Ellipsis),
];
const CHATS: &str = "Chats";
const CHATS_GROUP: &str = "sidebar-chats";
const SETTINGS: &str = "Settings";
const INSTANCE: &str = "Open instance, usage and MCP (current: Local)";

fn button(scene: Scene<'_>, name: &'static str, icon: IconName) -> AnyElement {
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, name);
    control(scene.registry, spec.clone(), icon_button(&spec, icon)).into_any_element()
}

/// The sidebar column.
pub(crate) fn sidebar(scene: Scene<'_>, cx: &App) -> Div {
    let toolbar = div()
        .h(px(40.))
        .flex()
        .items_center()
        .gap(px(2.))
        .px(px(8.))
        .children(TOOLBAR.map(|(name, icon)| button(scene, name, icon)));
    let column = div()
        .w(px(280.))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .bg(cx.theme().sidebar)
        .child(title_bar(scene))
        .child(resize_handle(scene, cx))
        .child(toolbar)
        .child(chats_row(scene))
        .children(status_line(scene, cx))
        .child(session_list(scene, cx))
        .child(footer(scene));
    region(scene.registry, Area::Sidebar, column)
}

/// The controls React draws in the window's title bar over the sidebar.
fn title_bar(scene: Scene<'_>) -> Div {
    // The title bar's and the Chats row's New session share a name; the reference numbers them.
    let new_session = ControlSpec::named(Area::Sidebar, Role::Button, NEW_SESSION).repeated(1);
    div()
        .h(px(48.))
        .flex()
        .items_center()
        .gap(px(2.))
        .px(px(8.))
        .children(TITLE_BAR.map(|(name, icon)| button(scene, name, icon)))
        .child(control(
            scene.registry,
            new_session.clone(),
            label_button(&new_session, Some(IconName::Plus), NEW_SESSION),
        ))
}

/// The resize handle on the sidebar's right edge, which also draws its border.
fn resize_handle(scene: Scene<'_>, cx: &App) -> Div {
    let resize = ControlSpec::named(Area::Sidebar, Role::Separator, RESIZE);
    control(
        scene.registry,
        resize.clone(),
        div()
            .id(resize.key.clone())
            .w(px(4.))
            .h_full()
            .flex()
            .justify_end()
            .child(div().w(px(1.)).h_full().bg(cx.theme().sidebar_border)),
    )
    .absolute()
    .top_0()
    .right_0()
    .h_full()
}

fn chats_row(scene: Scene<'_>) -> Div {
    let chats = ControlSpec::named(Area::Sidebar, Role::Button, CHATS);
    let add = ControlSpec::named(Area::Sidebar, Role::Button, NEW_SESSION)
        .repeated(2)
        .revealed(Reveal::OnHover(CHATS_GROUP.into()));
    div()
        .group(CHATS_GROUP)
        .flex()
        .items_center()
        .justify_between()
        .px(px(8.))
        .child(control(
            scene.registry,
            chats.clone(),
            label_button(&chats, None, CHATS),
        ))
        .child(control(
            scene.registry,
            add.clone(),
            icon_button(&add, IconName::Plus),
        ))
}

/// What the list shows instead of rows: server down, loading, or a failed load with retry.
fn status_line(scene: Scene<'_>, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    let text = |body: String| {
        div()
            .px(px(16.))
            .py(px(4.))
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(body)
    };
    match (scene.state.connection(), scene.state.sidebar()) {
        (ConnectionStatus::Failed(_) | ConnectionStatus::ServerExited(_), _) => {
            Some(text("Server not running".to_owned()).into_any_element())
        }
        (_, LoadStatus::Failed { message }) => {
            Some(
                div()
                    .flex()
                    .items_center()
                    .child(text(message.clone()))
                    .child(retry_button("sidebar-retry").on_click(
                        scene.on_click_dispatch(oc_state::Action::Retry(Region::Sidebar)),
                    ))
                    .into_any_element(),
            )
        }
        (_, LoadStatus::Loading) => Some(text("Loading sessions".to_owned()).into_any_element()),
        (_, LoadStatus::Ready) => None,
    }
}

fn footer(scene: Scene<'_>) -> Div {
    let instance = ControlSpec::named(Area::Sidebar, Role::Button, INSTANCE);
    div()
        .mt_auto()
        .h(px(48.))
        .flex()
        .items_center()
        .justify_between()
        .px(px(8.))
        .child(button(scene, SETTINGS, IconName::Settings))
        .child(control(
            scene.registry,
            instance.clone(),
            label_button(&instance, Some(IconName::HardDrive), "Local"),
        ))
}

#[cfg(test)]
#[path = "sidebar_test.rs"]
mod sidebar_test;
