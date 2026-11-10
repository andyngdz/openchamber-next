//! The 48 px header above the chat column, Work status panel and rail: session title, the
//! server state where React shows the context percentage, inert tools and window controls.

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, div,
    px,
};
use oc_state::{ConnectionStatus, ServerExit};

use super::{Scene, icon_button, label_button};
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control, region};

/// What the header shows: the empty state's tools, or the open session's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderMode {
    Empty,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowAction {
    Minimize,
    Maximize,
    Close,
}

/// One slot of the header, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderItem {
    Title,
    Tool(&'static str, IconName),
    /// Drag area; moves the window.
    Spacer,
    /// Server state as plain text, where the empty state has no context percentage.
    ServerState,
    /// Server state in the context percentage button.
    ContextUsage,
    Window(&'static str, IconName, WindowAction),
}

const CONTEXT_USAGE: &str = "Context usage";
/// The named landmarks the header's controls sit in, as the reference keys them.
const MAIN_NAVIGATION: &str = "Main navigation";
const WINDOW_CONTROL_GROUP: &str = "Window controls";
const TOGGLE_PANEL: HeaderItem = HeaderItem::Tool("Toggle work-status panel", IconName::PanelRight);
const WINDOW_CONTROLS: [HeaderItem; 3] = [
    HeaderItem::Window(
        "Minimize window",
        IconName::WindowMinimize,
        WindowAction::Minimize,
    ),
    HeaderItem::Window(
        "Maximize window",
        IconName::WindowMaximize,
        WindowAction::Maximize,
    ),
    HeaderItem::Window("Close window", IconName::WindowClose, WindowAction::Close),
];

fn items(mode: HeaderMode) -> Vec<HeaderItem> {
    let mut items = match mode {
        HeaderMode::Empty => vec![
            HeaderItem::Spacer,
            HeaderItem::ServerState,
            HeaderItem::Tool("Open a new Mini Chat window", IconName::Frame),
        ],
        HeaderMode::Session => vec![
            HeaderItem::Title,
            HeaderItem::Tool("Open session actions", IconName::Ellipsis),
            HeaderItem::Spacer,
            HeaderItem::ContextUsage,
            HeaderItem::Tool("Open in File Manager", IconName::Folder),
            HeaderItem::Tool("Run Auto-discover dev server", IconName::Play),
            HeaderItem::Tool("Open current session in Mini Chat", IconName::Frame),
        ],
    };
    items.push(TOGGLE_PANEL);
    items.extend(WINDOW_CONTROLS);
    items
}

/// The server state in words.
fn server_label(status: &ConnectionStatus) -> String {
    match status {
        ConnectionStatus::Starting => "Starting server".to_owned(),
        ConnectionStatus::Connected => "Connected".to_owned(),
        ConnectionStatus::Reconnecting { attempt } => format!("Reconnecting (attempt {attempt})"),
        ConnectionStatus::Failed(failure) => failure.message.clone(),
        ConnectionStatus::ServerExited(ServerExit::Code(code)) => {
            format!("Server exited with code {code}")
        }
        ConnectionStatus::ServerExited(ServerExit::Signal(signal)) => {
            format!("Server stopped by signal {signal}")
        }
        ConnectionStatus::ServerExited(ServerExit::Unknown) => "Server exited".to_owned(),
    }
}

/// The header row.
pub(crate) fn header(scene: Scene<'_>, cx: &App) -> Div {
    let mode = match scene.state.open() {
        Some(_) => HeaderMode::Session,
        None => HeaderMode::Empty,
    };
    let row = div()
        .h(px(48.))
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(4.))
        .pl(px(16.))
        .pr(px(8.))
        .children(
            items(mode)
                .into_iter()
                .map(|item| render_item(item, scene, cx)),
        );
    region(scene.registry, Area::Header, row)
}

fn render_item(item: HeaderItem, scene: Scene<'_>, cx: &App) -> AnyElement {
    let registry = scene.registry;
    let theme = cx.theme();
    let server = server_label(scene.state.connection());
    match item {
        HeaderItem::Title => {
            let title = scene
                .state
                .open_entry()
                .and_then(|entry| entry.title.clone())
                .unwrap_or_default();
            div().text_sm().truncate().child(title).into_any_element()
        }
        HeaderItem::Tool(name, icon) => tool(registry, name, icon),
        HeaderItem::Spacer => div()
            .flex_1()
            .h_full()
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .into_any_element(),
        HeaderItem::ServerState => div()
            .max_w(px(360.))
            .truncate()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(server)
            .into_any_element(),
        HeaderItem::ContextUsage => {
            let spec = ControlSpec::named(Area::Header, Role::Button, CONTEXT_USAGE)
                .under(&[MAIN_NAVIGATION]);
            let button = label_button(&spec, Some(IconName::LoaderCircle), server);
            control(registry, spec, button).into_any_element()
        }
        HeaderItem::Window(name, icon, action) => window_control(registry, name, icon, action),
    }
}

fn tool(registry: &LayoutRegistry, name: &'static str, icon: IconName) -> AnyElement {
    let spec = ControlSpec::named(Area::Header, Role::Button, name).under(&[MAIN_NAVIGATION]);
    control(registry, spec.clone(), icon_button(&spec, icon)).into_any_element()
}

fn window_control(
    registry: &LayoutRegistry,
    name: &'static str,
    icon: IconName,
    action: WindowAction,
) -> AnyElement {
    let spec = ControlSpec::named(Area::Header, Role::Button, name)
        .under(&[MAIN_NAVIGATION, WINDOW_CONTROL_GROUP]);
    let button = icon_button(&spec, icon).on_click(move |_, window, _| match action {
        WindowAction::Minimize => window.minimize_window(),
        WindowAction::Maximize => window.zoom_window(),
        WindowAction::Close => window.remove_window(),
    });
    control(registry, spec, button).into_any_element()
}

#[cfg(test)]
#[path = "header_test.rs"]
mod header_test;
