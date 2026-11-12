//! The tiles of the Work status panel: Context, Project, Context sources and Turn stats.

use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{
    AnyElement, Div, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, Styled, div,
    px,
};

use super::rows::{Header, finished_turn_rows, header_row};
use super::{CONTEXT_SOURCES, PanelMode, TURN_STATS, TURN_STATS_RATE, button, button_spec};
use crate::icons::{CustomIcon, RemixIcon};
use crate::layout::control;
use crate::theme::{DepthStyle, Tokens};
use crate::views::turn_stats::NO_FIGURE;
use crate::views::{Scene, project_name};

/// Glyph size inside a tile row.
pub(super) const GLYPH: f32 = 14.;
/// Figure shown where the state holds no value.
const UNKNOWN: &str = NO_FIGURE;

/// The three colors a tile row paints with.
#[derive(Clone, Copy)]
pub(super) struct Ink {
    pub(super) foreground: Hsla,
    pub(super) muted: Hsla,
}

impl Ink {
    pub(super) fn of(scene: Scene<'_>) -> Self {
        Self {
            foreground: scene.palette.foreground.into(),
            muted: scene.palette.muted_foreground.into(),
        }
    }
}

/// The one card: a tone-step tile holding every row, padded 10 12 12 12.
pub(super) fn card(tokens: &Tokens) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .px(px(12.))
        .pt(px(10.))
        .pb(px(12.))
        .tile(tokens, false)
}

/// A row of `height` px with 8 px between its parts.
fn line(height: f32) -> Div {
    div()
        .w_full()
        .h(px(height))
        .flex()
        .items_center()
        .gap(px(8.))
}

/// The card's title row: "Session" and the panel options button.
pub(super) fn title_row(scene: Scene<'_>) -> Div {
    line(28.)
        .child(
            div()
                .flex_1()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Session"),
        )
        .child(button(scene, "Choose sections", RemixIcon::Equalizer2))
}

/// A tile row glyph at the row icon size in `color`.
pub(super) fn glyph(icon: impl Into<Icon>, color: Hsla) -> Icon {
    icon.into().with_size(px(GLYPH)).text_color(color)
}

/// Context, Cache Hit, Project, branch and changed-files rows, once a session is open.
pub(super) fn session_rows(
    scene: Scene<'_>,
    mode: PanelMode,
    ink: Ink,
    tokens: &Tokens,
) -> Vec<AnyElement> {
    match mode {
        PanelMode::Empty => Vec::new(),
        PanelMode::Running | PanelMode::Finished => {
            let mut rows = context_rows(scene, ink, tokens);
            rows.extend(project_rows(scene, ink));
            rows
        }
    }
}

/// The usage ring row, an empty 3 px track and the Cache Hit row; the state holds no usage.
fn context_rows(scene: Scene<'_>, ink: Ink, tokens: &Tokens) -> Vec<AnyElement> {
    let spec = button_spec("Open context panel");
    let top = line(26.)
        .id(spec.key.clone())
        .text_color(ink.muted)
        .child(glyph(CustomIcon::ContextRing, ink.muted))
        .child(div().flex_1().child("Context"))
        .child(div().text_color(ink.foreground).child(UNKNOWN));
    let track = div()
        .w_full()
        .h(px(3.))
        .mt(px(2.))
        .mb(px(6.))
        .rounded(px(2.))
        .bg(tokens.depth.tile2);
    let cache = line(26.)
        .text_color(ink.muted)
        .child(div().flex_1().child("Cache Hit"))
        .child(div().text_color(ink.foreground).child(UNKNOWN));
    vec![
        control(scene.registry, spec, top)
            .w_full()
            .into_any_element(),
        track.into_any_element(),
        cache.into_any_element(),
    ]
}

/// The project's name, its branch and its changed files.
fn project_rows(scene: Scene<'_>, ink: Ink) -> Vec<AnyElement> {
    let name = line(26.)
        .mt(px(12.))
        .gap(px(6.))
        .child(div().font_weight(FontWeight::SEMIBOLD).child("Project"))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(ink.muted)
                .child(project_name(scene.state).to_owned()),
        );
    vec![
        name.into_any_element(),
        project_row(scene, "Open Git panel", RemixIcon::GitBranch, UNKNOWN, ink),
        project_row(
            scene,
            "Open changes",
            RemixIcon::FileEdit,
            "- files changed",
            ink,
        ),
    ]
}

/// A Project row: a muted glyph and a muted line of text.
fn project_row(
    scene: Scene<'_>,
    name: &'static str,
    icon: RemixIcon,
    text: &'static str,
    ink: Ink,
) -> AnyElement {
    let spec = button_spec(name);
    let row = line(26.)
        .id(spec.key.clone())
        .text_color(ink.muted)
        .child(glyph(icon, ink.muted))
        .child(div().flex_1().child(text));
    control(scene.registry, spec, row)
        .w_full()
        .into_any_element()
}

/// The Context sources row.
pub(super) fn context_sources(scene: Scene<'_>, ink: Ink) -> AnyElement {
    div()
        .w_full()
        .mt(px(10.))
        .child(header_row(
            scene,
            ink,
            Header {
                name: CONTEXT_SOURCES,
                icon: RemixIcon::Stack,
                label: "Context sources",
                chevron: RemixIcon::ArrowRightS,
                trailing: Some("3 skills"),
            },
        ))
        .into_any_element()
}

/// Turn stats: collapsed while a turn runs, open with its rows once it ended.
pub(super) fn turn_stats(scene: Scene<'_>, mode: PanelMode, ink: Ink) -> Vec<AnyElement> {
    let header = |name, chevron, trailing| {
        div()
            .w_full()
            .mt(px(10.))
            .child(header_row(
                scene,
                ink,
                Header {
                    name,
                    icon: RemixIcon::BarChart2,
                    label: TURN_STATS,
                    chevron,
                    trailing,
                },
            ))
            .into_any_element()
    };
    match mode {
        PanelMode::Empty => Vec::new(),
        PanelMode::Running => vec![header(TURN_STATS, RemixIcon::ArrowRightS, None)],
        PanelMode::Finished => {
            let mut rows = vec![header(
                TURN_STATS_RATE,
                RemixIcon::ArrowDownS,
                Some("- tok/s"),
            )];
            rows.extend(finished_turn_rows(scene, ink));
            rows
        }
    }
}

#[cfg(test)]
#[path = "work_status_tiles_test.rs"]
mod work_status_tiles_test;
