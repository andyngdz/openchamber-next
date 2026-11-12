//! The model picker's list: its sections, the row the keyboard is on, and its keys.

use gpui_kit::component::IndexPath;
use gpui_kit::component::input::InputState;
use gpui_kit::component::list::{ListDelegate, ListState};
use gpui_kit::{App, AppContext as _, Context, Entity, IntoElement, Window};
use oc_state::State;

use super::list::RowAction;
use super::list::{Section, header, row_item};
use super::model::SEARCH;
use super::{Pick, model_rows};
use crate::layout::LayoutRegistry;
use crate::views::{Dispatch, Intent};

/// The model picker's sections, filtered by the panel's search field.
pub(crate) struct ModelList {
    state: Entity<State>,
    dispatch: Dispatch,
    registry: LayoutRegistry,
    search: Entity<InputState>,
    query: String,
    selected: Option<IndexPath>,
}

impl ModelList {
    /// A delegate reading the shared state each frame; `search` drives its filter.
    pub(crate) fn new(
        state: Entity<State>,
        dispatch: Dispatch,
        registry: LayoutRegistry,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self {
            state,
            dispatch,
            registry,
            search: cx.new(|cx| InputState::new(window, cx).placeholder(SEARCH)),
            query: String::new(),
            selected: None,
        }
    }

    pub(super) fn sections(&self, cx: &App) -> Vec<Section> {
        model_rows::sections(self.state.read(cx), &self.query)
    }

    /// The panel's search field, which drives this list's filter.
    pub(crate) fn search(&self) -> Entity<InputState> {
        self.search.clone()
    }

    /// Drops the filter, so the list shows every section again.
    pub(super) fn clear_query(&mut self) {
        self.query.clear();
    }

    pub(super) fn set_query(&mut self, query: String, cx: &App) {
        self.query = query;
        self.selected = self.first_row(cx);
    }

    pub(super) fn first_row(&self, cx: &App) -> Option<IndexPath> {
        self.sections(cx)
            .iter()
            .enumerate()
            .find_map(|(section, entries)| {
                (!entries.rows.is_empty()).then(|| IndexPath::new(0).section(section))
            })
    }

    pub(super) fn initial_row(&self, cx: &App) -> Option<IndexPath> {
        let current = super::labels::selected_model(self.state.read(cx));
        for (section, entries) in self.sections(cx).iter().enumerate() {
            for (row, entry) in entries.rows.iter().enumerate() {
                let RowAction::Pick(Pick::Model {
                    provider_id,
                    model_id,
                }) = &entry.action
                else {
                    continue;
                };
                if current
                    .as_ref()
                    .is_some_and(|(provider, model)| provider == provider_id && model == model_id)
                {
                    return Some(IndexPath::new(row).section(section));
                }
            }
        }
        self.first_row(cx)
    }

    pub(super) fn step_row(
        &self,
        from: Option<IndexPath>,
        delta: isize,
        cx: &App,
    ) -> Option<IndexPath> {
        let rows: Vec<IndexPath> = self
            .sections(cx)
            .iter()
            .enumerate()
            .flat_map(|(section, entries)| {
                (0..entries.rows.len()).map(move |row| IndexPath::new(row).section(section))
            })
            .collect();
        let position = from.and_then(|from| rows.iter().position(|row| *row == from));
        wrapped(position, delta, rows.len()).and_then(|next| rows.get(next).copied())
    }

    pub(super) fn selected_pick(&self, cx: &App) -> Option<Pick> {
        let index = self.selected?;
        let sections = self.sections(cx);
        let row = sections.get(index.section)?.rows.get(index.row)?;
        match &row.action {
            RowAction::Pick(pick) => Some(pick.clone()),
            RowAction::Close => None,
        }
    }
}

impl ListDelegate for ModelList {
    type Item = gpui_kit::component::list::ListItem;

    fn sections_count(&self, cx: &App) -> usize {
        self.sections(cx).len().max(1)
    }

    fn items_count(&self, section: usize, cx: &App) -> usize {
        self.sections(cx)
            .get(section)
            .map_or(0, |section| section.rows.len())
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.sections(cx)
            .get(index.section)?
            .rows
            .get(index.row)
            .map(|row| row_item(&self.registry, &self.dispatch, row, cx))
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let sections = self.sections(cx);
        let wanted = sections.get(section)?.header.as_ref()?;
        Some(header(&self.registry, wanted).into_any_element())
    }

    fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected = index;
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        if let Some(pick) = self.selected_pick(cx) {
            (self.dispatch)(&Intent::Pick(pick), window, cx);
        } else {
            (self.dispatch)(&Intent::ClosePopover, window, cx);
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        (self.dispatch)(&Intent::ClosePopover, window, cx);
    }
}

fn wrapped(current: Option<usize>, delta: isize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let next = match current {
        Some(current) => (current as isize + delta).rem_euclid(count as isize) as usize,
        None if delta < 0 => count - 1,
        None => 0,
    };
    Some(next)
}

#[cfg(test)]
#[path = "model_list_test.rs"]
mod model_list_test;
