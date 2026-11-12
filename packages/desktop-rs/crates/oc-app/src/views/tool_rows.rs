//! Tool calls under a turn's Activity row. React draws each as a button named by the tool's
//! display name and what it ran on, so each row is a control with that key, `#n` added where
//! the name repeats in the chat column.

use gpui_kit::component::{ActiveTheme, Icon, Sizable, spinner::Spinner};
use gpui_kit::{AnyElement, App, Div, Hsla, IntoElement, ParentElement, Styled, div, px};
use oc_state::{ToolItem, ToolStatus};

use super::Scene;
use super::turns::{Block, ToolTitle, Turn, repeat_indexes, tool_status};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Role, control};

/// Height of one tool row, and the icon size at its start (`.tool` in the Session mockup).
const ROW_HEIGHT: f32 = 24.;
const ROW_ICON: f32 = 13.;

/// Every tool row of the column in drawing order, keyed by its title and, where the title
/// repeats in the column, its `#n`.
pub(super) fn tool_row_specs(turns: &[Turn<'_>]) -> Vec<ControlSpec> {
    let titles: Vec<String> = turns
        .iter()
        .flat_map(Turn::blocks)
        .flat_map(|block| match block {
            Block::Activity(tools) => tools,
            Block::Text(_) => Vec::new(),
        })
        .map(|tool| ToolTitle::of(tool).accessible_name())
        .collect();
    let indexes = repeat_indexes(&titles);
    titles
        .into_iter()
        .zip(indexes)
        .map(|(title, index)| {
            let spec = ControlSpec::named(Area::ChatColumn, Role::Button, title);
            match index {
                Some(index) => spec.repeated(index),
                None => spec,
            }
        })
        .collect()
}

/// Specs of the tool rows not drawn yet, consumed in drawing order.
pub(super) type ToolSpecs = std::vec::IntoIter<ControlSpec>;

/// One row per tool call, each a control keyed by its title, two pixels apart.
pub(super) fn activity(
    scene: Scene<'_>,
    tools: Vec<&ToolItem>,
    tool_specs: &mut ToolSpecs,
    cx: &App,
) -> Div {
    div().flex().flex_col().gap(px(2.)).children(
        tools
            .into_iter()
            .zip(tool_specs.by_ref())
            .map(|(tool, spec)| control(scene.registry, spec, tool_row(tool, cx))),
    )
}

/// The glyph that stands for a tool's kind.
fn tool_icon(name: &str) -> RemixIcon {
    match name {
        "glob" | "grep" | "codesearch" => RemixIcon::Search,
        "edit" | "write" | "patch" | "apply_patch" => RemixIcon::FileEdit,
        "bash" => RemixIcon::Terminal,
        "webfetch" | "websearch" => RemixIcon::Global,
        _ => RemixIcon::File,
    }
}

/// The icon at the row's start and the verb's colour: a spinner and the info colour for the
/// live step, the kind's glyph in muted (danger on error) and the foreground otherwise.
fn row_mark(tool: &ToolItem, cx: &App) -> (AnyElement, Hsla) {
    let theme = cx.theme();
    if tool.status == ToolStatus::Running {
        let spinner = Spinner::new()
            .icon(Icon::new(RemixIcon::Loader4))
            .with_size(px(ROW_ICON))
            .color(theme.info);
        return (spinner.into_any_element(), theme.info);
    }
    let color = if tool.status == ToolStatus::Error {
        theme.danger
    } else {
        theme.muted_foreground
    };
    let icon = Icon::new(tool_icon(&tool.name))
        .with_size(px(ROW_ICON))
        .text_color(color);
    (icon.into_any_element(), theme.foreground)
}

/// One tool call on one line: a 13 px icon, the verb, the target in the mono font and, on
/// the right, the status until the call is done. The live step is drawn in the info colour
/// with a spinner in place of its icon.
fn tool_row(tool: &ToolItem, cx: &App) -> Div {
    let theme = cx.theme();
    let title = ToolTitle::of(tool);
    let live = tool.status == ToolStatus::Running;
    let (mark, verb_color) = row_mark(tool, cx);
    div()
        .h(px(ROW_HEIGHT))
        .flex()
        .items_center()
        .gap(px(8.))
        .text_size(px(13.))
        .child(div().flex_none().child(mark))
        .child(
            div()
                .flex_none()
                .text_color(verb_color)
                .child(title.name.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme.mono_font_family.clone())
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .children(title.detail.map(str::to_owned)),
        )
        .children(tool_status(tool).filter(|_| !live).map(|status| {
            div()
                .flex_none()
                .text_size(px(12.))
                .text_color(if tool.status == ToolStatus::Error {
                    theme.danger
                } else {
                    theme.muted_foreground
                })
                .child(status)
        }))
}

#[cfg(test)]
#[path = "tool_rows_test.rs"]
mod tool_rows_test;
