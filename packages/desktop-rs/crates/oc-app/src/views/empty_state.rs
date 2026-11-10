//! The chat column with no session open: heading, Choose project, the composer and the
//! quick-action chips, placed as the reference measures them. A server that failed to start or
//! exited shows its error and a Retry that restarts it above, without moving the composer.

use gpui_kit::assets::IconName;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    AnyElement, App, Div, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{ConnectionStatus, ServerFailure};

use super::header::server_label;

use super::composer::{WIDTH, composer};
use super::{Intent, Scene, icon_button, label_button, retry_button};
use crate::layout::{Area, ControlSpec, Role, control};

/// Chat column width minus the composer width, halved: the composer's left edge.
const LEFT: f32 = (792. - WIDTH) / 2.;
/// Top of the heading block; heading, gap and Choose project above the composer add up to 100.
const TOP: f32 = 239.6;
/// The server error card sits above the heading and never reaches it.
const ERROR_TOP: f32 = 24.;
const ERROR_MAX_HEIGHT: f32 = TOP - ERROR_TOP - 16.;
const CHOOSE_PROJECT: &str = "Choose project";
const ADD_STARTER: &str = "Add a starter";
const CHIPS: [(&str, IconName); 9] = [
    ("Explore the codebase", IconName::Map),
    ("Catch me up", IconName::Calendar),
    ("Weigh my options", IconName::ChartPie),
    ("Start feature planning", IconName::FileText),
    ("Craft a Goal", IconName::Star),
    ("Schedule a Task", IconName::Calendar),
    ("Debug an issue", IconName::TriangleAlert),
    ("Review my changes", IconName::Search),
    ("Build an extension", IconName::LayoutDashboard),
];

fn chat_control(scene: Scene<'_>, name: &'static str, icon: IconName) -> AnyElement {
    let spec = ControlSpec::named(Area::ChatColumn, Role::Button, name);
    control(
        scene.registry,
        spec.clone(),
        label_button(&spec, Some(icon), name),
    )
    .into_any_element()
}

/// A quick-action chip: smaller and outlined, so a row holds as many as React's.
fn chip(scene: Scene<'_>, name: &'static str, icon: IconName) -> AnyElement {
    let spec = ControlSpec::named(Area::ChatColumn, Role::Button, name);
    let button = label_button(&spec, Some(icon), name).outline().xsmall();
    control(scene.registry, spec, button).into_any_element()
}

/// The empty state, filling the chat column.
pub(crate) fn empty_state(scene: Scene<'_>, draft: &Entity<TextareaState>, cx: &App) -> Div {
    let starter = ControlSpec::named(Area::ChatColumn, Role::Button, ADD_STARTER);
    let block = div()
        .absolute()
        .top(px(TOP))
        .left(px(LEFT))
        .w(px(WIDTH))
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(48.))
                .flex()
                .items_center()
                .justify_center()
                .text_3xl()
                .child("What are we working on?"),
        )
        .child(div().h(px(16.)))
        .child(div().h(px(28.)).flex().items_center().child(chat_control(
            scene,
            CHOOSE_PROJECT,
            IconName::ChevronDown,
        )))
        .child(div().h(px(8.)))
        .child(composer(scene, draft, cx))
        .child(
            div()
                .mt(px(16.))
                .flex()
                .flex_wrap()
                .justify_center()
                .gap(px(6.))
                .children(CHIPS.map(|(name, icon)| chip(scene, name, icon)))
                .child(control(
                    scene.registry,
                    starter.clone(),
                    icon_button(&starter, IconName::Plus),
                )),
        );
    div()
        .size_full()
        .children(server_down(scene, cx))
        .child(block)
}

/// The error card of a server that failed to start or exited.
fn server_down(scene: Scene<'_>, cx: &App) -> Option<AnyElement> {
    let connection = scene.state.connection();
    match connection {
        ConnectionStatus::Failed(failure) => Some(server_error(scene, failure, cx)),
        ConnectionStatus::ServerExited(_) => {
            let exited = ServerFailure {
                message: server_label(connection),
                ..ServerFailure::default()
            };
            Some(server_error(scene, &exited, cx))
        }
        ConnectionStatus::Starting
        | ConnectionStatus::Connected
        | ConnectionStatus::Reconnecting { .. } => None,
    }
}

/// The server failure with the output that explains it; Retry restarts the server. A tail the
/// failure has no lines for is left out.
fn server_error(scene: Scene<'_>, failure: &ServerFailure, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let tail = |title: &'static str, lines: &[String]| {
        (!lines.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(title),
                )
                .children(lines.iter().map(|line| {
                    div()
                        .font_family(theme.mono_font_family.clone())
                        .text_xs()
                        .child(line.clone())
                }))
        })
    };
    div()
        .absolute()
        .top(px(ERROR_TOP))
        .left(px(LEFT))
        .w(px(WIDTH))
        .max_h(px(ERROR_MAX_HEIGHT))
        .id("server-error")
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(12.))
        .rounded(px(12.))
        .border_1()
        .border_color(theme.danger)
        .child(
            div()
                .text_sm()
                .text_color(theme.danger)
                .child(failure.message.clone()),
        )
        .child(retry_button("server-retry").on_click(scene.on_click_intent(Intent::RestartServer)))
        .children(tail("Server output", &failure.stderr_tail))
        .children(tail("Server log", &failure.log_tail))
        .into_any_element()
}

#[cfg(test)]
#[path = "empty_state_test.rs"]
mod empty_state_test;
