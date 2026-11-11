//! The four picker popups, built on gpui-component's `Popover` and `List`/`PopupMenu`.
//!
//! `Popover` owns placement, outside-press dismissal, focus capture and restoration, and the
//! scroll-blocking surface. `List` owns the search field, filtering, rows, scrolling, and
//! keyboard navigation; `PopupMenu` owns the Thinking menu.

mod agent;
mod labels;
mod list;
mod model;
mod model_rows;
mod project;
mod thinking;

use gpui_kit::component::button::Button;
use gpui_kit::component::list::ListState;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::popover::{Popover, PopoverState};
use gpui_kit::{
    Anchor, AnyElement, App, AppContext as _, Context, Div, Entity, IntoElement, ParentElement,
    Styled, Window, div, px,
};
use oc_state::State;

use super::{Dispatch, Scene};
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};

pub(crate) use labels::{
    selected_agent_label, selected_model_label, selected_model_variants, selection_directory,
};
pub(crate) use types::{Pick, PopoverKind};

mod types;

const CHOOSE_PROJECT: &str = "Choose project";
/// The model trigger when no catalog model is selected.
pub(crate) const DEFAULT_MODEL: &str = "Default model";

/// The searchable lists and the Thinking menu, owned by the root view so their state survives
/// a render.
#[derive(Clone)]
pub(crate) struct PopoverLists {
    pub(crate) project: Entity<ListState<project::ProjectList>>,
    pub(crate) model: Entity<ListState<model::ModelList>>,
    pub(crate) agent: Entity<ListState<agent::AgentList>>,
    pub(crate) thinking: Entity<PopupMenu>,
}

impl PopoverLists {
    /// Creates the four popup states; their delegates read the shared state each frame.
    pub(crate) fn new(
        state: &Entity<State>,
        dispatch: Dispatch,
        registry: LayoutRegistry,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let project = cx.new(|cx| {
            ListState::new(
                project::ProjectList::new(state.clone(), dispatch.clone(), registry.clone()),
                window,
                cx,
            )
            .searchable(true)
        });
        let model = cx.new(|cx| {
            ListState::new(
                model::ModelList::new(state.clone(), dispatch.clone(), registry.clone()),
                window,
                cx,
            )
            .searchable(true)
        });
        let agent = cx.new(|cx| {
            ListState::new(
                agent::AgentList::new(state.clone(), dispatch.clone(), registry.clone()),
                window,
                cx,
            )
            .searchable(true)
        });
        let rows = thinking::rows(state.read(cx));
        let thinking = thinking::build(window, cx, rows, &dispatch, &registry);
        Self {
            project,
            model,
            agent,
            thinking,
        }
    }
}

/// The popup trigger and its panel, all owned by the `Popover` component.
pub(crate) fn trigger_control(scene: Scene<'_>, kind: PopoverKind, button: Button) -> Div {
    let spec = trigger_spec(kind, scene.state);
    control(scene.registry, spec, popover(scene, kind, button))
}

/// The control the `Popover` trigger is recorded under, for tests that press it.
pub(crate) fn trigger_spec(kind: PopoverKind, state: &State) -> ControlSpec {
    match kind {
        PopoverKind::Project => ControlSpec::named(Area::ChatColumn, Role::Button, CHOOSE_PROJECT),
        PopoverKind::Model | PopoverKind::Agent | PopoverKind::Thinking => {
            ControlSpec::named(Area::Composer, Role::Button, trigger_name(kind, state))
        }
    }
}

fn trigger_name(kind: PopoverKind, state: &State) -> String {
    match kind {
        PopoverKind::Project => CHOOSE_PROJECT.to_owned(),
        PopoverKind::Model => {
            selected_model_label(state).unwrap_or_else(|| DEFAULT_MODEL.to_owned())
        }
        PopoverKind::Agent => selected_agent_label(state).unwrap_or_else(|| "Build".to_owned()),
        PopoverKind::Thinking => {
            let label = state
                .current_selection()
                .and_then(|selection| selection.variant())
                .and_then(|choice| choice.variant());
            let effort = crate::views::model_names::effort_label(label);
            if state.open().is_some() {
                format!("Thinking: {effort}")
            } else {
                format!("Thinking: {effort}, last used with this model")
            }
        }
    }
}

fn popover(scene: Scene<'_>, kind: PopoverKind, button: Button) -> Popover {
    let state = scene.state_entity();
    let dispatch = scene.dispatch.clone();
    let registry = scene.registry.clone();
    let lists = scene.lists.clone();
    let open = scene.popover == Some(kind);
    let close = dispatch.clone();
    let open_change = move |open: &bool, window: &mut Window, cx: &mut App| {
        let intent = if *open {
            super::Intent::OpenPopover(kind)
        } else {
            super::Intent::ClosePopover
        };
        close(&intent, window, cx);
    };
    let content = move |_: &mut PopoverState, _: &mut Window, cx: &mut Context<PopoverState>| {
        content(kind, &state, &dispatch, &registry, &lists, cx)
    };
    let popover = Popover::new(format!("popover-{kind:?}"))
        .open(open)
        .on_open_change(open_change)
        .trigger(button)
        .content(content);
    match kind {
        PopoverKind::Project => popover
            .anchor(Anchor::TopLeft)
            .offset(px(4.))
            .p(px(0.))
            .w(px(project::WIDTH)),
        PopoverKind::Model => popover
            .anchor(Anchor::BottomRight)
            .offset(px(4.))
            .p(px(0.))
            .w(px(model::WIDTH))
            .max_h(px(model::MAX_HEIGHT)),
        PopoverKind::Agent => popover
            .anchor(Anchor::BottomRight)
            .offset(px(4.))
            .p(px(0.))
            .w(px(agent::WIDTH)),
        PopoverKind::Thinking => popover
            .appearance(false)
            .anchor(Anchor::BottomRight)
            .offset(px(4.)),
    }
}

fn content(
    kind: PopoverKind,
    state: &Entity<State>,
    dispatch: &Dispatch,
    registry: &LayoutRegistry,
    lists: &PopoverLists,
    cx: &App,
) -> AnyElement {
    let body = match kind {
        PopoverKind::Project => project::content(state, dispatch, registry, &lists.project, cx),
        PopoverKind::Model => model::content(state, dispatch, registry, &lists.model, cx),
        PopoverKind::Agent => agent::content(state, dispatch, registry, &lists.agent, cx),
        PopoverKind::Thinking => lists.thinking.clone().into_any_element(),
    };
    crate::layout::region(registry, Area::Popover, div().w_full().child(body)).into_any_element()
}

/// The Thinking menu rows for the current model.
pub(crate) fn thinking_rows(state: &State) -> Vec<(String, bool, Pick)> {
    thinking::rows(state)
}

/// Fills an existing Thinking menu with rows, keeping its identity.
pub(crate) fn thinking_items(
    menu: PopupMenu,
    rows: Vec<(String, bool, Pick)>,
    dispatch: &Dispatch,
    registry: &LayoutRegistry,
) -> PopupMenu {
    thinking::items(menu, rows, dispatch, registry)
}

#[cfg(test)]
#[path = "popover_test.rs"]
mod popover_test;
