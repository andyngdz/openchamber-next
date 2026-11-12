//! The 44 px header row of the chat card (session title, the server state where React shows
//! the context percentage, inert tools) and the window controls drawn over the top right.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, App, Div, Hsla, IntoElement, ParentElement, Styled, div, px};
use oc_state::{ConnectionStatus, ServerExit};

use super::window_chrome::{
    CONTROLS_WIDTH, STRIP_HEIGHT, WindowAction, drag_region, window_control,
};
use super::{Intent, Scene, icon_button, label_button, retry_button};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control, region};

/// What the header shows: the empty state's tools, or the open session's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderMode {
    Empty,
    Session,
}

/// One slot of the header, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderItem {
    Title,
    Tool(&'static str, RemixIcon),
    /// Space that pushes the items after it to the right.
    Spacer,
    /// Server state as plain text, where the empty state has no context percentage.
    ServerState,
    /// Server state in the context percentage button.
    ContextUsage,
    Window(&'static str, RemixIcon, WindowAction),
}

const CONTEXT_USAGE: &str = "Context usage";
/// The named landmarks the header's controls sit in, as the reference keys them.
pub(super) const MAIN_NAVIGATION: &str = "Main navigation";
const TOGGLE_PANEL: HeaderItem =
    HeaderItem::Tool("Toggle work-status panel", RemixIcon::ListIndefinite);
const WINDOW_CONTROLS: [HeaderItem; 3] = [
    HeaderItem::Window(
        "Minimize window",
        RemixIcon::Subtract,
        WindowAction::Minimize,
    ),
    HeaderItem::Window(
        "Maximize window",
        RemixIcon::CheckboxBlank,
        WindowAction::Maximize,
    ),
    HeaderItem::Window("Close window", RemixIcon::Close, WindowAction::Close),
];

fn items(mode: HeaderMode) -> Vec<HeaderItem> {
    let mut items = match mode {
        HeaderMode::Empty => vec![
            HeaderItem::Spacer,
            HeaderItem::ServerState,
            HeaderItem::Tool("Open a new Mini Chat window", RemixIcon::PictureInPicture2),
        ],
        HeaderMode::Session => vec![
            HeaderItem::Title,
            HeaderItem::Tool("Open session actions", RemixIcon::More),
            HeaderItem::Spacer,
            HeaderItem::ContextUsage,
            HeaderItem::Tool("Open in File Manager", RemixIcon::Folder),
            HeaderItem::Tool("Run Auto-discover dev server", RemixIcon::Scan2),
            HeaderItem::Tool(
                "Open current session in Mini Chat",
                RemixIcon::PictureInPicture2,
            ),
        ],
    };
    items.push(TOGGLE_PANEL);
    items.extend(WINDOW_CONTROLS);
    items
}

/// The server state in words.
pub(crate) fn server_label(status: &ConnectionStatus) -> String {
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
        .h(px(STRIP_HEIGHT))
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
                .filter(|item| !matches!(item, HeaderItem::Window(..)))
                .map(|item| render_item(item, scene, cx)),
        );
    region(scene.registry, Area::Header, drag_region(row))
}

/// The window controls, drawn over the top right of the window above every zone. They belong
/// to the header region's controls, after its tools.
pub(crate) fn window_controls(scene: Scene<'_>, cx: &App) -> Div {
    let controls = WINDOW_CONTROLS
        .into_iter()
        .map(|item| render_item(item, scene, cx));
    drag_region(
        div()
            .absolute()
            .top_0()
            .right_0()
            .w(px(CONTROLS_WIDTH))
            .h(px(STRIP_HEIGHT))
            .flex()
            .items_center()
            .justify_end()
            .gap(px(2.))
            .pr(px(8.))
            .children(controls),
    )
}

fn render_item(item: HeaderItem, scene: Scene<'_>, cx: &App) -> AnyElement {
    let registry = scene.registry;
    let theme = cx.theme();
    match item {
        HeaderItem::Title => title(scene),
        HeaderItem::Tool(name, icon) => {
            // "Toggle work-status panel" draws the foreground; the rest the muted foreground.
            let color = if name == "Toggle work-status panel" {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            tool_button(registry, name, icon, color)
        }
        HeaderItem::Spacer => spacer(),
        HeaderItem::ServerState => div()
            .max_w(px(360.))
            .truncate()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(server_label(scene.state.connection()))
            .into_any_element(),
        HeaderItem::ContextUsage => {
            let spec = ControlSpec::named(Area::Header, Role::Button, CONTEXT_USAGE)
                .under(&[MAIN_NAVIGATION]);
            // React draws its own `ContextRing` (`HeaderContextUsage.tsx`) before the label; it
            // is not ported yet, so nothing records it until the header unit draws it.
            let muted = theme.muted_foreground;
            let server = server_label(scene.state.connection());
            let button = label_button(registry, &spec, None, server, muted);
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(control(registry, spec, button))
                .children(restart_button(scene))
                .into_any_element()
        }
        HeaderItem::Window(name, icon, action) => window_control(registry, name, icon, action),
    }
}

/// The open session's title, or nothing in the empty state.
fn title(scene: Scene<'_>) -> AnyElement {
    let title = scene
        .state
        .open_entry()
        .and_then(|entry| entry.title.clone())
        .unwrap_or_default();
    div().text_sm().truncate().child(title).into_any_element()
}

/// The gap between the header's groups.
fn spacer() -> AnyElement {
    div().flex_1().h_full().into_any_element()
}

/// Restarts a server that exited or failed; the empty state shows its own Retry instead.
fn restart_button(scene: Scene<'_>) -> Option<AnyElement> {
    match scene.state.connection() {
        ConnectionStatus::Failed(_) | ConnectionStatus::ServerExited(_) => Some(
            retry_button("server-restart")
                .on_click(scene.on_click_intent(Intent::RestartServer))
                .into_any_element(),
        ),
        ConnectionStatus::Starting
        | ConnectionStatus::Connected
        | ConnectionStatus::Reconnecting { .. } => None,
    }
}

fn tool_button(
    registry: &LayoutRegistry,
    name: &'static str,
    icon: RemixIcon,
    color: Hsla,
) -> AnyElement {
    let spec = ControlSpec::named(Area::Header, Role::Button, name).under(&[MAIN_NAVIGATION]);
    control(
        registry,
        spec.clone(),
        icon_button(registry, &spec, icon, color),
    )
    .into_any_element()
}

#[cfg(test)]
#[path = "header_test.rs"]
mod header_test;
