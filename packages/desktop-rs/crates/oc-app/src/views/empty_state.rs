//! The chat column with no session open: heading, Choose project, the composer and the
//! quick-action chips, placed as the reference measures them. A server that failed to start or
//! exited shows its error and a Retry that restarts it above, without moving the composer.

use gpui_kit::component::input::TextareaState;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    AnyElement, App, Div, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use oc_state::{ConnectionStatus, ServerFailure};

use super::header::server_label;

use super::composer::{WIDTH, composer};
use super::popover::PopoverKind;
use super::{Intent, Scene, icon_button, label_button, retry_button};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Role, control};

/// Chat column width minus the composer width, halved: the composer's left edge.
const LEFT: f32 = (792. - WIDTH) / 2.;
/// Top of the heading block; heading, gaps and Choose project above the composer add up to
/// its 100 px (`DraftTargetSelectors.tsx:284`, the row's `mb-1.5`).
const TOP: f32 = 241.6;
/// `mb-1.5` between the Choose project row and the composer.
const GAP: f32 = 6.;
/// The server error card sits above the heading and never reaches it.
const ERROR_TOP: f32 = 24.;
const ERROR_MAX_HEIGHT: f32 = TOP - ERROR_TOP - 16.;
const CHOOSE_PROJECT: &str = "Choose project";
const ADD_STARTER: &str = "Add a starter";
const CHIPS: [(&str, RemixIcon); 9] = [
    ("Explore the codebase", RemixIcon::Compass3),
    ("Catch me up", RemixIcon::History),
    ("Weigh my options", RemixIcon::Scales3),
    ("Start feature planning", RemixIcon::Survey),
    ("Craft a Goal", RemixIcon::Target),
    ("Schedule a Task", RemixIcon::CalendarSchedule),
    ("Debug an issue", RemixIcon::Bug),
    ("Review my changes", RemixIcon::SearchEye),
    ("Build an extension", RemixIcon::Puzzle),
];

fn choose_project(scene: Scene<'_>, cx: &App) -> AnyElement {
    let spec = ControlSpec::named(Area::ChatColumn, Role::Button, CHOOSE_PROJECT);
    let button = label_button(&spec, Some(RemixIcon::ArrowDownS), CHOOSE_PROJECT)
        // `size="sm"` plus `h-7` on the trigger (`DraftTargetSelectors.tsx:330-333`: 1.75rem).
        .h(px(28.));
    // `px-0.5` on the Choose project row (`DraftTargetSelectors.tsx:284`).
    div()
        .h(px(28.))
        .px(px(2.))
        .flex()
        .items_center()
        .child(super::popover::trigger_control(
            scene,
            PopoverKind::Project,
            button,
            cx,
        ))
        .into_any_element()
}

/// A quick-action chip: smaller and outlined, so a row holds as many as React's.
fn chip(scene: Scene<'_>, name: &'static str, icon: RemixIcon) -> AnyElement {
    let spec = ControlSpec::named(Area::ChatColumn, Role::Button, name);
    let button = label_button(&spec, Some(icon), name).outline().xsmall();
    control(scene.registry, spec, button).into_any_element()
}

/// The empty state, filling the chat column.
pub(crate) fn empty_state(
    scene: Scene<'_>,
    draft: &Entity<TextareaState>,
    window: &mut Window,
    cx: &App,
) -> Div {
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
        .child(choose_project(scene, cx))
        .child(div().h(px(GAP)))
        .child(composer(scene, draft, window, cx))
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
                    icon_button(&starter, RemixIcon::Add),
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
