//! The model picker panel: search, the leading actions, and the section list.

use gpui_kit::component::IndexPath;
use gpui_kit::component::list::{List, ListDelegate, ListState};
use gpui_kit::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement, Styled, Task, Window, div, px,
};
use oc_state::{LoadStatus, State};

use super::list::RowAction;
use super::list::{Section, header, row_item, search};
use super::model_rows;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};
use crate::views::{Dispatch, Intent, RETRY, retry_button};

/// `w-[min(380px,calc(100vw-2rem))]` (`ModelControls.tsx:2407-2409`).
pub(super) const WIDTH: f32 = 380.;
/// The tallest menu the CSS lets the model picker take (`ModelControls.tsx:2407-2409`).
pub(super) const MAX_HEIGHT: f32 = 392.6;
const SEARCH: &str = "Search models";
const SEARCH_HEIGHT: f32 = 36.;
const ROW_HEIGHT: f32 = 30.;
const HEADER_HEIGHT: f32 = 29.;

/// The model picker's sections, filtered by the list's search field.
pub(crate) struct ModelList {
    state: Entity<State>,
    dispatch: Dispatch,
    registry: LayoutRegistry,
    query: String,
    selected: Option<IndexPath>,
}

impl ModelList {
    /// A delegate reading the shared state each frame.
    pub(crate) fn new(state: Entity<State>, dispatch: Dispatch, registry: LayoutRegistry) -> Self {
        Self {
            state,
            dispatch,
            registry,
            query: String::new(),
            selected: None,
        }
    }

    fn sections(&self, cx: &App) -> Vec<Section> {
        model_rows::sections(self.state.read(cx), &self.query)
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
            .map(|row| row_item(&self.registry, &self.dispatch, row))
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
        let Some(index) = self.selected else {
            return;
        };
        let sections = self.sections(cx);
        let Some(row) = sections
            .get(index.section)
            .and_then(|section| section.rows.get(index.row))
        else {
            return;
        };
        match &row.action {
            RowAction::Pick(pick) => (self.dispatch)(&Intent::Pick(pick.clone()), window, cx),
            RowAction::Close => (self.dispatch)(&Intent::ClosePopover, window, cx),
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        (self.dispatch)(&Intent::ClosePopover, window, cx);
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_owned();
        Task::ready(())
    }
}

/// The panel: the search list, or the catalog failure with Retry.
pub(super) fn content(
    state: &Entity<State>,
    dispatch: &Dispatch,
    registry: &LayoutRegistry,
    list: &Entity<ListState<ModelList>>,
    cx: &App,
) -> AnyElement {
    let state = state.read(cx);
    let directory = super::selection_directory(state);
    let failed = directory
        .as_deref()
        .and_then(|directory| state.catalog_load(directory))
        .and_then(|load| match load {
            LoadStatus::Failed { message } => Some(message.clone()),
            _ => None,
        });
    if let Some(message) = failed {
        return failure(registry, dispatch, &message, directory);
    }
    let sections = list.read(cx).delegate().sections(cx);
    let rows: usize = sections.iter().map(|section| section.rows.len()).sum();
    let headers = sections
        .iter()
        .filter(|section| section.header.is_some() && !section.rows.is_empty())
        .count();
    let height =
        (SEARCH_HEIGHT + rows as f32 * ROW_HEIGHT + headers as f32 * HEADER_HEIGHT).min(MAX_HEIGHT);
    let list = List::new(list).search_placeholder(SEARCH).w_full();
    let panel = search(registry, Role::Combobox, SEARCH, list).h(px(height));
    div().w_full().child(panel).into_any_element()
}

fn failure(
    registry: &LayoutRegistry,
    dispatch: &Dispatch,
    message: &str,
    directory: Option<String>,
) -> AnyElement {
    let retry = dispatch.clone();
    let retry = retry_button("catalog-retry").on_click(move |_, window, cx| {
        let intent = match &directory {
            Some(directory) => Intent::LoadCatalog(directory.clone()),
            None => Intent::ClosePopover,
        };
        retry(&intent, window, cx);
    });
    let spec = ControlSpec::named(Area::Popover, Role::Button, RETRY);
    div()
        .p(px(12.))
        .w_full()
        .child(message.to_owned())
        .child(control(registry, spec, retry))
        .into_any_element()
}

#[cfg(test)]
#[path = "model_test.rs"]
mod model_test;
