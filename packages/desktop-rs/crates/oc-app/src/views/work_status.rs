//! The Work status panel: one 300 px card with the Session, Project, Context sources and Turn
//! stats sections. Everything here is inert; values are placeholders.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use oc_state::{SessionStatus, State};

use super::turn_stats::{StatRow, stat_rows};
use super::turns::turns;
use super::{Scene, icon_button, label_button, project_name};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Role, control, region};
use crate::theme::{DARK, card};

/// The card's name; every control in it sits under it.
const WORK_STATUS: &str = "Work status";
const TURN_STATS: &str = "Turn stats";
/// Name of the Turn stats header once a turn has a rate; the reference normalizes the figure.
const TURN_STATS_RATE: &str = "Turn stats <rate> tok/s";
const CONTEXT_SOURCES: &str = "Context sources 3 skills";

/// Which sections the card shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelMode {
    /// No session open: Session and Context sources only.
    Empty,
    /// A turn runs: every section, Turn stats collapsed.
    Running,
    /// The last turn ended: Turn stats open with its rows.
    Finished,
}

fn panel_mode(state: &State) -> PanelMode {
    let Some(open) = state.open() else {
        return PanelMode::Empty;
    };
    let ended = turns(&open.messages)
        .last()
        .is_some_and(|turn| turn.meta().is_some());
    match (state.open_status(), ended) {
        (SessionStatus::Idle | SessionStatus::Error, true) => PanelMode::Finished,
        (SessionStatus::Busy, _) | (_, false) => PanelMode::Running,
    }
}

/// `(name, icon, label)` of the section buttons above Context sources.
fn session_rows() -> [(&'static str, RemixIcon, &'static str); 3] {
    [
        ("Open context panel", RemixIcon::DonutChart, "Context"),
        ("Open Git panel", RemixIcon::GitBranch, "Branch"),
        ("Open changes", RemixIcon::FileEdit, "Changes"),
    ]
}

/// The 324 px column holding the card.
pub(crate) fn work_status(scene: Scene<'_>, cx: &App) -> Div {
    let theme = cx.theme();
    let mode = panel_mode(scene.state);
    let card_body = div()
        .w(px(300.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(12.))
        .rounded(px(12.))
        .border_1()
        .border_color(theme.border)
        .bg(card(DARK))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(section("Session", cx))
                .child(button(scene, "Choose sections", RemixIcon::Equalizer2)),
        )
        .children(project_section(scene, mode, cx))
        .child(context_sources(scene, cx))
        .children(turn_stats(scene, mode));
    let column = div()
        .w(px(324.))
        .h_full()
        .flex_none()
        .pt(px(16.))
        .pl(px(8.))
        .child(card_body);
    region(scene.registry, Area::WorkStatusPanel, column)
}

fn section(label: &'static str, cx: &App) -> Div {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(label)
}

/// Context, the project and its Git rows, once a session is open.
fn project_section(scene: Scene<'_>, mode: PanelMode, cx: &App) -> Vec<AnyElement> {
    match mode {
        PanelMode::Empty => Vec::new(),
        PanelMode::Running | PanelMode::Finished => {
            let [context, git, changes] = session_rows().map(|row| labeled(scene, row));
            vec![
                context,
                section("Project", cx).into_any_element(),
                div()
                    .text_sm()
                    .child(project_name(scene.state).to_owned())
                    .into_any_element(),
                git,
                changes,
            ]
        }
    }
}

fn context_sources(scene: Scene<'_>, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(labeled(
            scene,
            (CONTEXT_SOURCES, RemixIcon::Stack, "Context sources"),
        ))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("3 skills"),
        )
}

/// Turn stats: collapsed while a turn runs, open with its rows once it ended.
fn turn_stats(scene: Scene<'_>, mode: PanelMode) -> Vec<AnyElement> {
    match mode {
        PanelMode::Empty => Vec::new(),
        PanelMode::Running => vec![labeled(
            scene,
            (TURN_STATS, RemixIcon::BarChart2, TURN_STATS),
        )],
        PanelMode::Finished => std::iter::once(labeled(
            scene,
            (TURN_STATS_RATE, RemixIcon::BarChart2, TURN_STATS),
        ))
        .chain(finished_turn_rows(scene))
        .collect(),
    }
}

fn button(scene: Scene<'_>, name: &'static str, icon: RemixIcon) -> AnyElement {
    let spec = ControlSpec::named(Area::WorkStatusPanel, Role::Button, name).under(&[WORK_STATUS]);
    control(scene.registry, spec.clone(), icon_button(&spec, icon)).into_any_element()
}

fn labeled(
    scene: Scene<'_>,
    (name, icon, label): (&'static str, RemixIcon, &'static str),
) -> AnyElement {
    let spec = ControlSpec::named(Area::WorkStatusPanel, Role::Button, name).under(&[WORK_STATUS]);
    control(
        scene.registry,
        spec.clone(),
        label_button(&spec, Some(icon), label),
    )
    .into_any_element()
}

/// A Turn stats row; React leaves it unnamed, so it is named by its visible label.
fn finished_turn_rows(scene: Scene<'_>) -> Vec<AnyElement> {
    let Some(open) = scene.state.open() else {
        return Vec::new();
    };
    let all = turns(&open.messages);
    let Some(turn) = all.last() else {
        return Vec::new();
    };
    stat_rows(turn)
        .into_iter()
        .map(|row| stat_row(scene, row))
        .collect()
}

/// A Turn stats row, named by its label; its figure is data.
fn stat_row(scene: Scene<'_>, row: StatRow) -> AnyElement {
    let spec =
        ControlSpec::named(Area::WorkStatusPanel, Role::Generic, row.label).under(&[WORK_STATUS]);
    let line = div()
        .id(spec.key.clone())
        .w_full()
        .flex()
        .justify_between()
        .text_xs()
        .child(row.label)
        .child(row.value);
    control(scene.registry, spec, line)
        .w_full()
        .into_any_element()
}

#[cfg(test)]
#[path = "work_status_test.rs"]
mod work_status_test;
