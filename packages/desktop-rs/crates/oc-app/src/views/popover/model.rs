//! The model picker panel: search, the leading actions, and the section list.

#[path = "model_rows.rs"]
mod rows;

use gpui_kit::component::input::Input;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    App, Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{LoadStatus, ModelSections, model_sections};

use super::common::{control, frame};
use crate::layout::{Area, ControlSpec, Role, control as control_element};
use crate::views::{Intent, Scene};
use rows::{Section, model_row, option_row_flat, provider_header, section_header};

/// `w-[min(380px,calc(100vw-2rem))]` (`ModelControls.tsx:2407-2409`).
pub(super) const WIDTH: f32 = 380.;
const SEARCH: &str = "Search models";
const ADD_PROVIDER: &str = "Add new provider";

/// The model picker panel.
pub(super) fn panel(scene: Scene<'_>, cx: &App, max_height: f32) -> Div {
    let directory = super::selection_directory(scene.state);
    if let Some(LoadStatus::Failed { message }) = directory
        .as_deref()
        .and_then(|directory| scene.state.catalog_load(directory))
    {
        return failure(scene, cx, message.clone());
    }
    let query = scene.search.model.read(cx).value().to_string();
    let (Some(settings), Some(directory)) = (scene.state.settings(), directory) else {
        return frame(scene, cx, Some(WIDTH), div());
    };
    let Some(catalog) = scene.state.catalog(&directory) else {
        return frame(scene, cx, Some(WIDTH), div());
    };
    let sections = model_sections(catalog, settings, &query);
    frame(
        scene,
        cx,
        Some(WIDTH),
        div()
            .flex()
            .flex_col()
            .h(px(max_height))
            .child(search_row(scene, cx))
            .child(
                div()
                    .id("model-list")
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(list_children(scene, &sections)),
            ),
    )
}

fn failure(scene: Scene<'_>, cx: &App, message: String) -> Div {
    let retry = div().child(message).child(control_element(
        scene.registry,
        ControlSpec::named(Area::Popover, Role::Button, crate::views::RETRY),
        div()
            .id("catalog-retry")
            .child(crate::views::RETRY)
            .on_click(
                scene.on_click_intent(match super::selection_directory(scene.state) {
                    Some(directory) => Intent::LoadCatalog(directory),
                    None => Intent::ClosePopover,
                }),
            ),
    ));
    frame(scene, cx, Some(WIDTH), retry)
}

fn search_row(scene: Scene<'_>, cx: &App) -> Div {
    let input = Input::new(&scene.search.model)
        .appearance(false)
        .small()
        .w_full();
    let add = control_element(
        scene.registry,
        ControlSpec::named(Area::Popover, Role::Button, ADD_PROVIDER),
        div()
            .id("add-provider")
            .child(ADD_PROVIDER)
            .on_click(scene.on_click_intent(Intent::ClosePopover)),
    );
    // `flex items-center gap-2 px-2 py-1 border-b` (`ModelPickerList.tsx:1307`).
    div()
        .px(px(8.))
        .py(px(4.))
        .border_b_1()
        .border_color(cx.theme().border)
        .flex()
        .items_center()
        .gap(px(8.))
        .child(control(scene, Role::Combobox, SEARCH, &[], input))
        .child(add)
}

fn list_children(scene: Scene<'_>, sections: &ModelSections) -> Vec<Div> {
    let mut children = vec![option_row_flat(
        scene,
        "Run on several models",
        &[rows::MODELS_GROUP],
    )];
    if !sections.favorites.is_empty() {
        children.push(section_header(scene, Section::Favorites));
        children.extend(
            sections
                .favorites
                .iter()
                .map(|entry| model_row(scene, entry, Section::Favorites, rows::FAVORITES)),
        );
    }
    if !sections.recent.is_empty() {
        children.push(section_header(scene, Section::Recent));
        children.extend(
            sections
                .recent
                .iter()
                .map(|entry| model_row(scene, entry, Section::Recent, rows::RECENT)),
        );
    }
    for provider in &sections.providers {
        children.push(provider_header(
            scene,
            &provider.name,
            &provider.provider_id,
        ));
        children.extend(
            provider
                .models
                .iter()
                .map(|entry| model_row(scene, entry, Section::Provider, &provider.name)),
        );
    }
    children
}

#[cfg(test)]
#[path = "model_test.rs"]
mod model_test;
