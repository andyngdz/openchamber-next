//! The project picker: Chat and the ranked projects, over `List`.

use gpui_kit::component::IndexPath;
use gpui_kit::component::list::{List, ListDelegate, ListState};
use gpui_kit::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement, Styled, Task, Window, div, px,
};
use oc_state::{Project, State, ranked_projects};

use super::list::RowAction;
use super::list::{Row, search};
use crate::layout::{LayoutRegistry, Role};
use crate::views::{Dispatch, Intent};

/// `w-72` = 18rem at the 16px root (`DraftTargetSelectors.tsx:373`).
pub(super) const WIDTH: f32 = 288.;
const SEARCH: &str = "Search projects...";
const PROJECT_GROUP: &str = "Project";
const SEARCH_HEIGHT: f32 = 32.;
const ROW_HEIGHT: f32 = 34.2;

/// The project picker's rows, filtered by the list's search field.
pub(crate) struct ProjectList {
    state: Entity<State>,
    dispatch: Dispatch,
    registry: LayoutRegistry,
    query: String,
    selected: Option<IndexPath>,
}

impl ProjectList {
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

    fn rows(&self, cx: &App) -> Vec<Row> {
        let mut rows = vec![project_row("Chat".to_owned(), super::Pick::Chat)];
        if let Some(settings) = self.state.read(cx).settings() {
            rows.extend(
                ranked_projects(settings, &self.query)
                    .into_iter()
                    .filter_map(|project| {
                        let id = project.id()?.to_owned();
                        Some(project_row(label(project), super::Pick::Project(id)))
                    }),
            );
        }
        rows
    }
}

fn project_row(name: String, pick: super::Pick) -> Row {
    Row {
        name: name.clone(),
        label: name,
        icon: None,
        context: None,
        selected: false,
        role: Role::Option,
        ancestors: vec![PROJECT_GROUP.to_owned()],
        action: RowAction::Pick(pick),
        star: None,
        described: None,
        repeat: None,
        star_under_row: true,
    }
}

impl ListDelegate for ProjectList {
    type Item = gpui_kit::component::list::ListItem;

    fn items_count(&self, _section: usize, cx: &App) -> usize {
        self.rows(cx).len()
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.rows(cx)
            .get(index.row)
            .map(|row| super::list::row_item(&self.registry, &self.dispatch, row, cx))
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
        let rows = self.rows(cx);
        let Some(row) = rows.get(index.row) else {
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

fn label(project: &Project) -> String {
    project
        .label()
        .map(str::to_owned)
        .or_else(|| {
            project.path().map(|path| {
                path.trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(path)
                    .to_owned()
            })
        })
        .unwrap_or_default()
}

/// The popped-up list: its search field, then the ranked rows.
pub(super) fn content(
    _state: &Entity<State>,
    _dispatch: &Dispatch,
    registry: &LayoutRegistry,
    list: &Entity<ListState<ProjectList>>,
    cx: &App,
) -> AnyElement {
    let rows = list.read(cx).delegate().rows(cx);
    let height = SEARCH_HEIGHT + rows.len() as f32 * ROW_HEIGHT;
    let list = List::new(list).search_placeholder(SEARCH).w_full();
    let panel = search(registry, Role::Combobox, SEARCH, list).h(px(height));
    div().w_full().child(panel).into_any_element()
}

#[cfg(test)]
#[path = "project_test.rs"]
mod project_test;
