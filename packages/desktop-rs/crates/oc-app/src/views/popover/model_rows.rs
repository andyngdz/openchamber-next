//! One model row and the section headers around it.

use gpui_kit::assets::IconName;
use gpui_kit::{
    Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};
use oc_state::{ModelEntry, SelectionAction};

use crate::layout::Role;
use crate::views::Intent;
use crate::views::Scene;
use crate::views::model_names::compact_tokens;
use crate::views::popover::common::{control, hidden_control};
use crate::views::popover::{Pick, selected_model};

/// The list's accessibility group, `ModelPickerList`'s named list.
pub(super) const MODELS_GROUP: &str = "Models";
/// The Favorites group and header (`chat.modelControls.favorites`).
pub(super) const FAVORITES: &str = "Favorites";
/// The Recent group and header (`chat.modelControls.recent`).
pub(super) const RECENT: &str = "Recent";

/// Which section a row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    /// A starred model.
    Favorites,
    /// A recently picked model.
    Recent,
    /// A model of one provider's open section.
    Provider,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Self::Favorites => FAVORITES,
            Self::Recent => RECENT,
            Self::Provider => MODELS_GROUP,
        }
    }

    fn header(self) -> &'static str {
        match self {
            Self::Favorites => "FAVORITES",
            Self::Recent => "RECENT",
            Self::Provider => MODELS_GROUP,
        }
    }

    fn shows_provider(self) -> bool {
        !matches!(self, Self::Provider)
    }
}

/// The Favorites or Recent section header (`PICKER_SECTION_HEADER_CLASS`).
pub(super) fn section_header(scene: Scene<'_>, section: Section) -> Div {
    let header = div()
        .h(px(29.))
        .px(px(8.))
        .flex()
        .items_center()
        .text_xs()
        .child(section.header());
    control(
        scene,
        Role::Button,
        section.header(),
        &[MODELS_GROUP, section.label()],
        header,
    )
}

/// One provider's header (`ModelPickerList.tsx:1100-1110`).
pub(super) fn provider_header(scene: Scene<'_>, provider_name: &str, provider_id: &str) -> Div {
    let name = format!("{provider_id} logo {}", provider_name.to_uppercase());
    let header = div()
        .h(px(29.))
        .px(px(8.))
        .flex()
        .items_center()
        .text_xs()
        .child(name.clone());
    control(
        scene,
        Role::Button,
        name,
        &[MODELS_GROUP, provider_name],
        header,
    )
}

/// The leading "Run on several models" row.
pub(super) fn option_row_flat(scene: Scene<'_>, name: &'static str, ancestors: &[&str]) -> Div {
    let row = div()
        .id(name)
        .h(px(30.))
        .px(px(8.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .text_sm()
        .child(name);
    control(
        scene,
        Role::Option,
        name,
        ancestors,
        row.on_click(scene.on_click_intent(Intent::ClosePopover)),
    )
}

/// One model row with its star toggle.
pub(super) fn model_row(
    scene: Scene<'_>,
    entry: &ModelEntry,
    section: Section,
    group: &str,
) -> Div {
    let row_name = row_name(scene, entry, section);
    let favorite = scene.state.settings().is_some_and(|settings| {
        settings.favorite_models.iter().any(|reference| {
            reference.provider_id == entry.provider_id && reference.model_id == entry.model_id
        })
    });
    let label = if favorite { "Unfavorite" } else { "Favorite" };
    let star_ancestors = [MODELS_GROUP, group, row_name.as_str()];
    let star = hidden_control(
        scene,
        Role::Button,
        label,
        &star_ancestors,
        div()
            .id(label)
            .child(if favorite {
                IconName::StarFill
            } else {
                IconName::Star
            })
            .on_click(scene.on_click_intent(Intent::ApplySelection(
                SelectionAction::ToggleFavoriteModel {
                    provider_id: entry.provider_id.clone(),
                    model_id: entry.model_id.clone(),
                },
            ))),
    );
    let row = div()
        .id(row_name.clone())
        .h(px(30.))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(6.))
        .child(visible_name(entry, section))
        .child(div().flex_1())
        .child(star);
    control(
        scene,
        Role::Option,
        row_name.clone(),
        &[MODELS_GROUP, group],
        row.on_click(scene.on_click_intent(Intent::Pick(Pick::Model {
            provider_id: entry.provider_id.clone(),
            model_id: entry.model_id.clone(),
        }))),
    )
}

fn row_name(scene: Scene<'_>, entry: &ModelEntry, section: Section) -> String {
    let mut name = visible_name(entry, section);
    if is_selected(scene, entry) && !entry.variants.is_empty() {
        let variant = scene
            .state
            .current_selection()
            .and_then(|selection| selection.variant())
            .and_then(|choice| choice.variant())
            .unwrap_or("default");
        name.push(' ');
        name.push_str(variant);
    }
    name
}

fn visible_name(entry: &ModelEntry, section: Section) -> String {
    let mut name = crate::views::model_names::model_display_name(&entry.name, &entry.model_id);
    if section.shows_provider() {
        name.push(' ');
        name.push_str(&entry.provider_name);
    }
    if let Some(context) = entry.context {
        name.push(' ');
        name.push_str(&compact_tokens(context));
    }
    name
}

fn is_selected(scene: Scene<'_>, entry: &ModelEntry) -> bool {
    selected_model(scene.state).is_some_and(|(provider_id, model_id)| {
        provider_id == entry.provider_id && model_id == entry.model_id
    })
}

#[cfg(test)]
#[path = "model_rows_test.rs"]
mod model_rows_test;
