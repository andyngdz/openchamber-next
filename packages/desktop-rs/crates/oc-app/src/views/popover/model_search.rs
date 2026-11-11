//! The model picker's own search field: focus, filtering, keys and confirmation.
//!
//! `List` draws its search field itself and offers no slot for an action beside it, while
//! React places `Add new provider` in that row. The model panel therefore owns the field and
//! drives the same `ListState` the list renders, so every section can carry a header.

use gpui_kit::component::IndexPath;
use gpui_kit::component::input::InputState;
use gpui_kit::component::list::ListState;
use gpui_kit::{App, Context, Entity, KeyDownEvent, ScrollStrategy, Window};

use super::PopoverLists;
use super::model_list::ModelList;
use crate::views::{Dispatch, Intent};

/// A key the model picker handles while its search field holds focus.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerKey {
    Up,
    Down,
    Enter,
    Escape,
}

impl PickerKey {
    /// The picker key a platform key name stands for, if it is one.
    fn of(key: &str) -> Option<Self> {
        const KEYS: [(&str, PickerKey); 4] = [
            ("up", PickerKey::Up),
            ("down", PickerKey::Down),
            ("enter", PickerKey::Enter),
            ("escape", PickerKey::Escape),
        ];
        KEYS.iter()
            .find_map(|(name, picker)| (*name == key).then_some(*picker))
    }
}

/// Focuses the model picker's search field, clearing the filter it had applied.
pub(crate) fn open_model_picker(lists: &PopoverLists, window: &mut Window, cx: &mut App) {
    lists.model.update(cx, |list, cx| focus(list, window, cx));
}

/// The model panel's search field, whose edits filter the list.
pub(crate) fn model_search_field(lists: &PopoverLists, cx: &App) -> Entity<InputState> {
    lists.model.read(cx).delegate().search()
}

/// Applies the search field's text as the list's filter.
pub(crate) fn filter_model_list(
    lists: &PopoverLists,
    search: &Entity<InputState>,
    window: &mut Window,
    cx: &mut App,
) {
    let search_text = search.read(cx).value();
    let query = trimmed(&search_text);
    lists.model.update(cx, |list, cx| {
        list.delegate_mut().set_query(query, cx);
        let index = list.delegate().first_row(cx);
        list.set_selected_index(index, window, cx);
        list.scroll_to_item(IndexPath::default(), ScrollStrategy::Top, window, cx);
        cx.notify();
    });
}

/// Up, Down, Enter and Escape while the search field holds focus.
pub(super) fn search_keys(
    list: &Entity<ListState<ModelList>>,
    dispatch: &Dispatch,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(key) = PickerKey::of(event.keystroke.key.as_str()) else {
        return;
    };
    if key == PickerKey::Escape {
        dispatch(&Intent::ClosePopover, window, cx);
        window.prevent_default();
        cx.stop_propagation();
        return;
    }
    if key == PickerKey::Enter {
        if let Some(pick) = list.read(cx).delegate().selected_pick(cx) {
            dispatch(&Intent::Pick(pick), window, cx);
        }
        window.prevent_default();
        cx.stop_propagation();
        return;
    }
    let delta = if key == PickerKey::Down { 1 } else { -1 };
    list.update(cx, |list, cx| {
        let next = list.delegate().step_row(list.selected_index(), delta, cx);
        list.set_selected_index(next, window, cx);
        if let Some(next) = next {
            list.scroll_to_item(next, ScrollStrategy::Nearest, window, cx);
        }
        cx.notify();
    });
    window.prevent_default();
    cx.stop_propagation();
}

fn focus(
    list: &mut ListState<ModelList>,
    window: &mut Window,
    cx: &mut Context<ListState<ModelList>>,
) {
    let search = list.delegate().search();
    search.update(cx, |input, cx| {
        input.set_value("", window, cx);
        input.focus(window, cx);
    });
    list.delegate_mut().clear_query();
    let index = list.delegate().initial_row(cx);
    list.set_selected_index(index, window, cx);
    if let Some(index) = index {
        list.scroll_to_item(index, ScrollStrategy::Top, window, cx);
    }
    cx.notify();
}

/// The search text without the whitespace React ignores.
fn trimmed(value: &str) -> String {
    value.trim().to_owned()
}

#[cfg(test)]
#[path = "model_search_test.rs"]
mod model_search_test;
