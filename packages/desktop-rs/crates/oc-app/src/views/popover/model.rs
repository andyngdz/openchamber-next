//! The model picker panel: the search row with its action, the leading action, and the list.

use gpui_kit::component::input::{InputGroup, InputGroupAddon, InputGroupInput, InputState};
use gpui_kit::component::list::{List, ListState};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::{
    AnyElement, App, Div, Entity, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use oc_state::{LoadStatus, State};

use super::list::row_item;
use super::model_list::ModelList;
use super::model_rows;
use super::model_search;
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, LayoutRegistry, Role, control};
use crate::views::{Dispatch, Intent, RETRY, retry_button};

/// `w-[min(380px,calc(100vw-2rem))]` (`ModelControls.tsx:2407-2409`).
pub(super) const WIDTH: f32 = 380.;
/// The tallest menu the CSS lets the model picker take (`ModelControls.tsx:2407-2409`).
pub(super) const MAX_HEIGHT: f32 = 392.6;
/// The model picker's search field, `labels.searchPlaceholder`.
pub(super) const SEARCH: &str = "Search models";
/// `px-2 py-1.5` around the field, then its 28px frame (`ModelPickerList.tsx:2130`).
const SEARCH_ROW_HEIGHT: f32 = 36.;
const SEARCH_HEIGHT: f32 = 28.;
/// The leading action's row: `px-2 py-1.5` and the meta line (`ModelPickerList.tsx:960`).
const LEADING_HEIGHT: f32 = 30.;
const ROW_HEIGHT: f32 = 30.;
const HEADER_HEIGHT: f32 = 29.;

/// The panel: the search row, the leading action and the sections, or the failure with Retry.
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
    let delegate = list.read(cx).delegate();
    let sections = delegate.sections(cx);
    let rows: usize = sections.iter().map(|section| section.rows.len()).sum();
    let headers = sections
        .iter()
        .filter(|section| section.header.is_some() && !section.rows.is_empty())
        .count();
    let height = panel_height(rows, headers);
    let search = delegate.search();
    let panel = div()
        .w_full()
        .h(px(height))
        .flex()
        .flex_col()
        .child(search_row(registry, &search, dispatch, list, cx))
        .child(leading_row(registry, dispatch, cx))
        .child(div().flex_1().min_h_0().child(List::new(list).w_full()));
    div().w_full().child(panel).into_any_element()
}

/// The panel's height for its rows and headers, capped at the reference's tallest menu.
fn panel_height(rows: usize, headers: usize) -> f32 {
    (SEARCH_ROW_HEIGHT + LEADING_HEIGHT + rows as f32 * ROW_HEIGHT + headers as f32 * HEADER_HEIGHT)
        .min(MAX_HEIGHT)
}

fn search_row(
    registry: &LayoutRegistry,
    search: &Entity<InputState>,
    dispatch: &Dispatch,
    list: &Entity<ListState<ModelList>>,
    cx: &App,
) -> Div {
    let spec = ControlSpec::named(Area::Popover, Role::Combobox, SEARCH);
    let group = InputGroup::new("model-search")
        .small()
        .h(px(SEARCH_HEIGHT))
        .input(InputGroupInput::new(search).aria_label(SEARCH))
        .addon(
            InputGroupAddon::new("model-search-icon")
                .child(Icon::new(RemixIcon::Search).text_color(cx.theme().muted_foreground)),
        );
    let keys = list.clone();
    let close = dispatch.clone();
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(8.))
        .py(px(4.))
        .child(control(registry, spec, group).flex_1())
        .child(add_provider(registry, dispatch))
        .capture_key_down(move |event, window, cx| {
            model_search::search_keys(&keys, &close, event, window, cx)
        })
}

fn add_provider(registry: &LayoutRegistry, dispatch: &Dispatch) -> Div {
    let spec = ControlSpec::named(Area::Popover, Role::Button, model_rows::ADD_PROVIDER);
    let close = dispatch.clone();
    // `text-muted-foreground` on the icon and the label (`ModelPickerList.tsx`).
    let muted: gpui_kit::Hsla = crate::theme::palette().muted_foreground.into();
    let button = super::super::label_button(
        registry,
        &spec,
        Some((RemixIcon::Add, muted)),
        model_rows::ADD_PROVIDER,
        muted,
    )
    .compact()
    .accessibility_label(model_rows::ADD_PROVIDER)
    .on_click(move |_, window, cx| close(&Intent::ClosePopover, window, cx));
    control(registry, spec, button)
}

fn leading_row(registry: &LayoutRegistry, dispatch: &Dispatch, cx: &App) -> impl IntoElement {
    let close = dispatch.clone();
    row_item(registry, dispatch, &model_rows::leading_row(), cx)
        .on_click(move |_, window, cx| close(&Intent::ClosePopover, window, cx))
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
