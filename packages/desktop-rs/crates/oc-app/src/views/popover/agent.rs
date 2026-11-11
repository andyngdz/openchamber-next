//! The agent menu: a search field and the visible primary agents, over `List`.

use gpui_kit::component::IndexPath;
use gpui_kit::component::list::{List, ListDelegate, ListState};
use gpui_kit::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement, Styled, Task, Window, div, px,
};
use oc_state::{SelectionAction, State, ranked_agents};

use super::list::RowAction;
use super::list::{Row, Star, StarKind, search};
use crate::layout::{LayoutRegistry, Role};
use crate::views::{Dispatch, Intent, model_names::agent_display_name};

/// `w-[min(280px,calc(100vw-2rem))]` (`ModelControls.tsx:2894`).
pub(super) const WIDTH: f32 = 280.;
const SEARCH: &str = "Search agents";
/// `chat.modelControls.agentFavoriteAdd` (`en.ts:4003`).
const STAR_LABEL: &str = "Add to favorites. With favorites, Tab cycles through them only";
const SEARCH_HEIGHT: f32 = 36.;
const ROW_HEIGHT: f32 = 28.;

/// The agent picker's rows, filtered by the list's search field.
pub(crate) struct AgentList {
    state: Entity<State>,
    dispatch: Dispatch,
    registry: LayoutRegistry,
    query: String,
    selected: Option<IndexPath>,
}

impl AgentList {
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
        let Some(directory) = super::selection_directory(self.state.read(cx)) else {
            return Vec::new();
        };
        let state = self.state.read(cx);
        let Some(catalog) = state.catalog(&directory) else {
            return Vec::new();
        };
        let settings = state.settings();
        ranked_agents(catalog, &self.query)
            .into_iter()
            .enumerate()
            .map(|(index, agent)| {
                let favorite = settings.is_some_and(|settings| {
                    settings.favorite_agents.iter().any(|id| id == &agent.id)
                });
                let star = Star {
                    label: if favorite {
                        "Remove from favorites"
                    } else {
                        STAR_LABEL
                    },
                    action: SelectionAction::ToggleFavoriteAgent(agent.id.clone()),
                    kind: if favorite {
                        StarKind::Filled
                    } else {
                        StarKind::Outline
                    },
                    shown: true,
                };
                Row {
                    name: STAR_LABEL.to_owned(),
                    label: agent_display_name(&agent.display_name, &agent.id),
                    icon: None,
                    context: None,
                    selected: false,
                    role: Role::Menuitem,
                    ancestors: Vec::new(),
                    action: RowAction::Pick(super::Pick::Agent(agent.id.clone())),
                    star: Some(star),
                    described: Some("menuitem"),
                    repeat: Some(index + 1),
                    star_under_row: false,
                }
            })
            .collect()
    }
}

impl ListDelegate for AgentList {
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
        if let RowAction::Pick(pick) = &row.action {
            (self.dispatch)(&Intent::Pick(pick.clone()), window, cx);
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

/// The popped-up list: its search field, then the ranked agent rows.
pub(super) fn content(
    _state: &Entity<State>,
    _dispatch: &Dispatch,
    registry: &LayoutRegistry,
    list: &Entity<ListState<AgentList>>,
    cx: &App,
) -> AnyElement {
    let rows = list.read(cx).delegate().rows(cx);
    let height = SEARCH_HEIGHT + rows.len() as f32 * ROW_HEIGHT + 8.;
    let list = List::new(list).search_placeholder(SEARCH).w_full();
    let panel = search(registry, Role::Textbox, SEARCH, list).h(px(height));
    div().w_full().child(panel).into_any_element()
}

#[cfg(test)]
#[path = "agent_test.rs"]
mod agent_test;
