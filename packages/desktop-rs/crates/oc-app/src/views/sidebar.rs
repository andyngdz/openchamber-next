//! The 264 px sidebar: title-bar controls, toolbar, Chats, the session groups and the footer.
//! Live: session rows open their session. Everything else is inert.

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{
    AnyElement, App, Div, Hsla, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use oc_state::Region;

use super::sidebar_list::session_list;
use super::window_chrome::{STRIP_HEIGHT, drag_region};
use super::{Intent, Scene, draw_text, icon_button, label_button, retry_button};
use crate::icons::RemixIcon;
use crate::layout::draw::remix;
use crate::layout::{Area, ControlSpec, Role, control, region};

/// Title-bar controls, left to right.
/// The sidebar toggle's name.
const OPEN_SESSIONS: &str = "Open sessions";
const TITLE_BAR: [(&str, RemixIcon); 2] = [
    ("Open OpenChamber menu", RemixIcon::Menu2),
    (OPEN_SESSIONS, RemixIcon::LayoutLeft),
];
pub(crate) const NEW_SESSION: &str = "New session";
const RESIZE: &str = "Resize left panel";
const TOOLBAR: [(&str, RemixIcon); 4] = [
    ("Search sessions", RemixIcon::Search),
    ("Add project", RemixIcon::FolderAdd),
    ("Display mode", RemixIcon::Equalizer2),
    ("More", RemixIcon::More),
];
const SETTINGS: &str = "Settings";
const INSTANCE: &str = "Open instance, usage and MCP (current: Local)";
/// The label the instance chip shows.
const INSTANCE_LABEL: &str = "Local";

/// Side of a sidebar icon button.
const BUTTON_SIZE: f32 = 28.;
/// Column padding, left and right.
const COLUMN_PADDING: f32 = 8.;
/// Space between the controls of a row.
const CONTROL_GAP: f32 = 4.;
const TEXT_SIZE: f32 = 13.;
const FOOTER_HEIGHT: f32 = 36.;
const CHIP_HEIGHT: f32 = 26.;
const CHIP_TEXT_SIZE: f32 = 12.;
const CHIP_GAP: f32 = 6.;
const CHIP_ICON_SIZE: f32 = 14.;

/// A 28 px ghost icon button in the muted colour.
fn button(scene: Scene<'_>, name: &'static str, icon: RemixIcon) -> AnyElement {
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, name);
    // Electron draws the open-sessions toggle in the foreground, the other buttons muted.
    let color = if name == OPEN_SESSIONS {
        scene.palette.foreground.into()
    } else {
        scene.palette.muted_foreground.into()
    };
    control(
        scene.registry,
        spec.clone(),
        icon_button(scene.registry, &spec, icon, color)
            .w(px(BUTTON_SIZE))
            .h(px(BUTTON_SIZE)),
    )
    .into_any_element()
}

/// The sidebar column.
pub(crate) fn sidebar(scene: Scene<'_>, cx: &App) -> Div {
    let column = div()
        .w(px(264.))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .px(px(COLUMN_PADDING))
        .pb(px(COLUMN_PADDING))
        .child(drag_region(title_bar(scene)))
        .child(resize_handle(scene))
        .child(toolbar(scene))
        .children(create_line(scene, cx))
        .child(session_list(scene, cx))
        .child(footer(scene));
    region(scene.registry, Area::Sidebar, column)
}

/// The controls React draws in the window's title bar over the sidebar.
fn title_bar(scene: Scene<'_>) -> Div {
    // The title bar's and the Chats row's New session share a name; the reference numbers them.
    let new_session = ControlSpec::named(Area::Sidebar, Role::Button, NEW_SESSION).repeated(1);
    div()
        .h(px(STRIP_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(CONTROL_GAP))
        .children(TITLE_BAR.map(|(name, icon)| button(scene, name, icon)))
        .child(control(scene.registry, new_session.clone(), {
            let foreground = scene.palette.foreground.into();
            label_button(
                scene.registry,
                &new_session,
                Some((RemixIcon::ChatNew, foreground)),
                NEW_SESSION,
                foreground,
            )
            .h(px(BUTTON_SIZE))
            .text_size(px(TEXT_SIZE))
            .on_click(scene.on_click_intent(Intent::NewSession))
        }))
}

/// The four icon buttons under the title strip.
fn toolbar(scene: Scene<'_>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(CONTROL_GAP))
        .pt(px(2.))
        .pb(px(6.))
        .children(TOOLBAR.map(|(name, icon)| button(scene, name, icon)))
}

/// The resize handle on the sidebar's right edge. It draws no line: the chat card's edge
/// separates the zones.
fn resize_handle(scene: Scene<'_>) -> Div {
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
            .child(div().w(px(1.)).h_full()),
    )
    .absolute()
    .top_0()
    .right_0()
    .h_full()
}

/// A failed New session, with a Retry that creates again.
fn create_line(scene: Scene<'_>, cx: &App) -> Option<Div> {
    let message = scene.state.region_error(Region::NewSession)?;
    Some(
        div()
            .flex()
            .items_center()
            .child(
                div()
                    .px(px(16.))
                    .py(px(4.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(message.to_owned()),
            )
            .child(
                retry_button("new-session-retry")
                    .on_click(scene.on_click_dispatch(oc_state::Action::Retry(Region::NewSession))),
            ),
    )
}

/// The footer: Settings at the left, the instance chip at the right.
fn footer(scene: Scene<'_>) -> Div {
    div()
        .mt_auto()
        .h(px(FOOTER_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(CONTROL_GAP))
        .child(button(scene, SETTINGS, RemixIcon::Settings3))
        .child(div().flex_1())
        .child(instance_chip(scene))
}

/// The instance chip: a hard-drive icon and `Local`, in the muted colour.
fn instance_chip(scene: Scene<'_>) -> AnyElement {
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, INSTANCE);
    let muted = Hsla::from(scene.palette.muted_foreground);
    let hover = Hsla::from(scene.palette.hover);
    draw_text(scene.registry, &spec, INSTANCE_LABEL, muted);
    scene
        .registry
        .record_icon(&spec.key, remix(RemixIcon::HardDrive2, muted));
    let chip = div()
        .id(spec.key.clone())
        .h(px(CHIP_HEIGHT))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(CHIP_GAP))
        .rounded(px(6.))
        .text_size(px(CHIP_TEXT_SIZE))
        .text_color(muted)
        .hover(|style| style.bg(hover))
        .child(
            Icon::new(RemixIcon::HardDrive2)
                .with_size(px(CHIP_ICON_SIZE))
                .text_color(muted),
        )
        .child(INSTANCE_LABEL);
    control(scene.registry, spec, chip).into_any_element()
}

#[cfg(test)]
#[path = "sidebar_test.rs"]
mod sidebar_test;
