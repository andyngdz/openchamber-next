//! The Work status panel: separate tone-step tiles on the glass for Context, Project, Context
//! sources and Turn stats. Everything here is inert; a figure the state does not hold shows `-`.

use gpui_kit::{AnyElement, App, Div, IntoElement, ParentElement, Styled, div, px};
use oc_state::{SessionStatus, State};

use super::turns::turns;
use super::{Scene, icon_button};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Role, control, region};
use crate::theme::Tokens;
use tiles::{Ink, card, context_sources, session_rows, title_row, turn_stats};

#[path = "work_status_rows.rs"]
mod rows;
#[path = "work_status_tiles.rs"]
mod tiles;

/// The card's name; every control in it sits under it.
pub(super) const WORK_STATUS: &str = "Work status";
pub(super) const TURN_STATS: &str = "Turn stats";
/// Name of the Turn stats header once a turn has a rate; the reference normalizes the figure.
pub(super) const TURN_STATS_RATE: &str = "Turn stats <rate> tok/s";
pub(super) const CONTEXT_SOURCES: &str = "Context sources 3 skills";

/// Which sections the card shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelMode {
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

/// The 320 px column holding the card.
pub(crate) fn work_status(scene: Scene<'_>, cx: &App) -> Div {
    let mode = panel_mode(scene.state);
    let tokens = Tokens::of(cx);
    let ink = Ink::of(scene);
    let card = card(tokens)
        .child(title_row(scene))
        .children(session_rows(scene, mode, ink, tokens))
        .child(context_sources(scene, ink))
        .children(turn_stats(scene, mode, ink));
    let column = div()
        .w(px(320.))
        .flex_1()
        .min_h_0()
        .pt(px(12.))
        .px(px(8.))
        .pb(px(8.))
        .text_size(px(13.))
        .text_color(ink.foreground)
        .child(card);
    region(scene.registry, Area::WorkStatusPanel, column)
}

/// The spec of a button under the Work status card.
pub(super) fn button_spec(name: &'static str) -> ControlSpec {
    ControlSpec::named(Area::WorkStatusPanel, Role::Button, name).under(&[WORK_STATUS])
}

fn button(scene: Scene<'_>, name: &'static str, icon: RemixIcon) -> AnyElement {
    let spec = button_spec(name);
    let muted = scene.palette.muted_foreground.into();
    control(
        scene.registry,
        spec.clone(),
        icon_button(scene.registry, &spec, icon, muted)
            .w(px(24.))
            .h(px(24.)),
    )
    .into_any_element()
}

#[cfg(test)]
#[path = "work_status_test.rs"]
mod work_status_test;
