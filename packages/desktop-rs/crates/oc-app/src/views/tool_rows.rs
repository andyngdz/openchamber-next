//! Tool calls under a turn's Activity row. React draws each as a button named by the tool's
//! display name and what it ran on, so each row is a control with that key, `#n` added where
//! the name repeats in the chat column.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Div, FontWeight, ParentElement, Styled, div, px};
use oc_state::ToolItem;

use super::Scene;
use super::turns::{Block, ToolTitle, Turn, repeat_indexes, tool_status};
use crate::layout::{Area, ControlSpec, Role, control};

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

/// The Activity row and one row per tool call under it, each a control keyed by its title.
pub(super) fn activity(
    scene: Scene<'_>,
    tools: Vec<&ToolItem>,
    tool_specs: &mut ToolSpecs,
    cx: &App,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Activity"),
        )
        .children(
            tools
                .into_iter()
                .zip(tool_specs.by_ref())
                .map(|(tool, spec)| control(scene.registry, spec, tool_row(tool, cx))),
        )
}

/// One tool call: the display name, what it ran on, and its status until it is done.
fn tool_row(tool: &ToolItem, cx: &App) -> Div {
    let theme = cx.theme();
    let title = ToolTitle::of(tool);
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_xs()
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child(title.name.to_owned()),
        )
        .children(title.detail.map(|detail| {
            div()
                .text_color(theme.muted_foreground)
                .child(detail.to_owned())
        }))
        .children(
            tool_status(tool).map(|status| div().text_color(theme.muted_foreground).child(status)),
        )
}

#[cfg(test)]
#[path = "tool_rows_test.rs"]
mod tool_rows_test;
