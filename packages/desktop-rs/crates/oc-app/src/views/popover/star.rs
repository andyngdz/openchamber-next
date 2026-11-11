//! The star toggle a picker row carries.

use gpui_kit::{
    App, Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, Window, div,
    px,
};
use oc_state::SelectionAction;

use super::list::Row;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Reveal, Role, control as control_element};
use crate::views::Intent;

/// Which star a row shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StarKind {
    /// The model or agent is starred.
    Filled,
    /// The row can be starred.
    Outline,
}

/// A row's star toggle.
pub(super) struct Star {
    pub(super) label: &'static str,
    pub(super) action: SelectionAction,
    pub(super) kind: StarKind,
    pub(super) shown: bool,
}

impl Star {
    /// A model's favorite toggle, filled while the model is starred.
    pub(super) fn model(entry: &oc_state::ModelEntry, kind: StarKind) -> Self {
        let starred = matches!(kind, StarKind::Filled);
        Self {
            label: if starred { "Unfavorite" } else { "Favorite" },
            action: SelectionAction::ToggleFavoriteModel {
                provider_id: entry.provider_id.clone(),
                model_id: entry.model_id.clone(),
            },
            kind,
            shown: starred,
        }
    }
}

/// The row's star, recorded under the row so a click keeps its place in the tree.
pub(super) fn suffix(
    registry: LayoutRegistry,
    dispatch: super::Dispatch,
    row: &Row,
    star: &Star,
) -> impl Fn(&mut Window, &mut App) -> Div + 'static {
    let mut parts: Vec<&str> = row.ancestors.iter().map(String::as_str).collect();
    if row.star_under_row {
        parts.push(row.name.as_str());
    }
    let ancestors = super::list::ancestors(parts.into_iter());
    let label = star.label;
    let action = star.action.clone();
    let kind = star.kind;
    let mut spec = ControlSpec::named(Area::Popover, Role::Button, label).under(&ancestors);
    if let Some(index) = row.repeat {
        spec = spec.repeated(index);
    }
    let spec = if star.shown {
        spec
    } else {
        spec.revealed(Reveal::Hidden)
    };
    move |_, _| {
        let dispatch = dispatch.clone();
        let action = action.clone();
        control_element(
            &registry,
            spec.clone(),
            div()
                .id(label)
                .w(px(16.))
                .h(px(16.))
                .child(match kind {
                    StarKind::Filled => gpui_kit::assets::IconName::StarFill,
                    StarKind::Outline => gpui_kit::assets::IconName::Star,
                })
                .on_click(move |_, window, cx| {
                    dispatch(&Intent::ApplySelection(action.clone()), window, cx);
                    cx.stop_propagation();
                }),
        )
        .w(px(16.))
        .h(px(16.))
    }
}

#[cfg(test)]
#[path = "star_test.rs"]
mod star_test;
