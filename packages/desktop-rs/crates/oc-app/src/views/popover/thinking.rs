//! The Thinking menu: Default and the model's levels, built on `PopupMenu`.

use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::{App, Entity, IntoElement, ParentElement, Window, div, px};
use oc_state::State;

use super::Pick;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};
use crate::views::{Dispatch, Intent};

const DEFAULT: &str = "default";
const HINTS: &str = "↑↓ navigate    Ctrl + Shift + T switch";
const WIDTH: f32 = 227.29;

/// Builds the Thinking menu for the current model, with the active level checked.
pub(super) fn build(
    window: &mut Window,
    cx: &mut App,
    rows: Vec<(String, bool, Pick)>,
    dispatch: &Dispatch,
    registry: &LayoutRegistry,
) -> Entity<PopupMenu> {
    let dispatch = dispatch.clone();
    let registry = registry.clone();
    PopupMenu::build(window, cx, move |menu, _, _| {
        items(menu, rows, &dispatch, &registry)
    })
}

/// Fills a menu with the Thinking rows and the footer hints.
pub(super) fn items(
    mut menu: PopupMenu,
    rows: Vec<(String, bool, Pick)>,
    dispatch: &Dispatch,
    registry: &LayoutRegistry,
) -> PopupMenu {
    // The reference width is a floor only: gpui-kit indents a label further than the
    // reference does, and a capped menu let the hints run past its edge.
    menu = menu.min_w(px(WIDTH)).label("Thinking");
    for (label, checked, pick) in rows {
        let dispatch = dispatch.clone();
        let pick = pick.clone();
        let registry = registry.clone();
        let name = label.clone();
        menu = menu.item(
            PopupMenuItem::element(move |_, _| {
                control(
                    &registry,
                    ControlSpec::named(Area::Popover, Role::Menuitem, name.clone()),
                    div().child(name.clone()),
                )
                .into_any_element()
            })
            .checked(checked)
            .on_click(move |_, window, cx| {
                dispatch(&Intent::Pick(pick.clone()), window, cx);
            }),
        );
    }
    menu.label(HINTS)
}

/// The menu's rows: Default, then the selected model's levels.
pub(super) fn rows(state: &State) -> Vec<(String, bool, Pick)> {
    let current = state
        .current_selection()
        .and_then(|selection| selection.variant())
        .and_then(|choice| choice.variant())
        .map(str::to_owned);
    let mut rows = vec![(DEFAULT.to_owned(), current.is_none(), Pick::Variant(None))];
    rows.extend(
        super::selected_model_variants(state)
            .into_iter()
            .map(|variant| {
                let checked = current.as_deref() == Some(variant.as_str());
                let label = variant.to_lowercase();
                (label, checked, Pick::Variant(Some(variant)))
            }),
    );
    rows
}

#[cfg(test)]
#[path = "thinking_test.rs"]
mod thinking_test;
