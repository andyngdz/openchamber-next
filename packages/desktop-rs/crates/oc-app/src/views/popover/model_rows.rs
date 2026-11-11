//! The model picker's rows and sections.

use oc_state::{ModelEntry, ModelSections, SelectionAction, State, model_sections};

use super::list::RowAction;
use super::list::{Header, Row, Section, Star, StarKind};
use crate::layout::Role;
use crate::views::model_names::compact_tokens;
pub(super) const MODELS_GROUP: &str = "Models";
const ADD_PROVIDER: &str = "Add new provider";
const RUN_ON_SEVERAL: &str = "Run on several models";
const FAVORITES: &str = "Favorites";
const RECENT: &str = "Recent";

/// Whether a row's star is filled.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Favorite {
    /// The model is starred.
    Yes,
    /// The model can be starred.
    No,
}

impl Favorite {
    fn is_favorite(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// Whether a row is the session's current model.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Selected {
    /// This row is the current model.
    Yes,
    /// This row is another model.
    No,
}

impl Selected {
    fn is_selected(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// Which group a row belongs to, which decides whether its label shows the provider.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SectionKind {
    Favorites,
    Recent,
    Provider,
}

impl SectionKind {
    fn shows_provider(self) -> bool {
        !matches!(self, Self::Provider)
    }
}

/// The panel's sections in drawn order, filtered by `query`.
pub(super) fn sections(state: &State, query: &str) -> Vec<Section> {
    let mut sections = leading_sections();
    let Some(found) = found_sections(state, query) else {
        return sections;
    };
    sections.push(group(
        "FAVORITES",
        FAVORITES,
        &found.favorites,
        SectionKind::Favorites,
        state,
    ));
    sections.push(group(
        "RECENT",
        RECENT,
        &found.recent,
        SectionKind::Recent,
        state,
    ));
    sections.extend(
        found
            .providers
            .iter()
            .map(|provider| provider_section(provider, state)),
    );
    sections
}

fn leading_sections() -> Vec<Section> {
    vec![Section {
        header: None,
        rows: vec![
            flat_row(ADD_PROVIDER, Role::Button, Vec::new()),
            flat_row(RUN_ON_SEVERAL, Role::Option, vec![MODELS_GROUP.to_owned()]),
        ],
    }]
}

fn found_sections(state: &State, query: &str) -> Option<ModelSections> {
    let settings = state.settings()?;
    let directory = super::selection_directory(state)?;
    let catalog = state.catalog(&directory)?;
    Some(model_sections(catalog, settings, query))
}

fn provider_section(provider: &oc_state::ProviderSection, state: &State) -> Section {
    let name = format!(
        "{} logo {}",
        provider.provider_id,
        provider.name.to_uppercase()
    );
    Section {
        header: Some(Header {
            name,
            ancestors: vec![MODELS_GROUP.to_owned(), provider.name.clone()],
        }),
        rows: provider
            .models
            .iter()
            .map(|entry| model_row(entry, &provider.name, SectionKind::Provider, state))
            .collect(),
    }
}

fn group(
    header_name: &str,
    label: &str,
    entries: &[ModelEntry],
    kind: SectionKind,
    state: &State,
) -> Section {
    Section {
        header: Some(Header {
            name: header_name.to_owned(),
            ancestors: vec![MODELS_GROUP.to_owned(), label.to_owned()],
        }),
        rows: entries
            .iter()
            .map(|entry| model_row(entry, label, kind, state))
            .collect(),
    }
}

fn flat_row(name: &str, role: Role, ancestors: Vec<String>) -> Row {
    Row {
        name: name.to_owned(),
        visible: name.to_owned(),
        role,
        ancestors,
        action: RowAction::Close,
        star: None,
        described: None,
        repeat: None,
        star_under_row: true,
    }
}

fn model_row(entry: &ModelEntry, group: &str, kind: SectionKind, state: &State) -> Row {
    let favorite = favorite_state(state, entry);
    let selected = if is_selected(state, entry) {
        Selected::Yes
    } else {
        Selected::No
    };
    Row {
        name: row_name(entry, kind, selected, state),
        visible: visible_name(entry, kind),
        role: Role::Option,
        ancestors: vec![MODELS_GROUP.to_owned(), group.to_owned()],
        action: RowAction::Pick(super::Pick::Model {
            provider_id: entry.provider_id.clone(),
            model_id: entry.model_id.clone(),
        }),
        star: Some(model_star(entry, favorite)),
        described: None,
        repeat: None,
        star_under_row: true,
    }
}

fn favorite_state(state: &State, entry: &ModelEntry) -> Favorite {
    let favorite = state.settings().is_some_and(|settings| {
        settings.favorite_models.iter().any(|reference| {
            reference.provider_id == entry.provider_id && reference.model_id == entry.model_id
        })
    });
    if favorite {
        Favorite::Yes
    } else {
        Favorite::No
    }
}

fn is_selected(state: &State, entry: &ModelEntry) -> bool {
    state
        .current_selection()
        .and_then(|selection| selection.model())
        .is_some_and(|key| key.provider_id() == entry.provider_id && key.id() == entry.model_id)
}

fn model_star(entry: &ModelEntry, favorite: Favorite) -> Star {
    Star {
        label: if favorite.is_favorite() {
            "Unfavorite"
        } else {
            "Favorite"
        },
        action: SelectionAction::ToggleFavoriteModel {
            provider_id: entry.provider_id.clone(),
            model_id: entry.model_id.clone(),
        },
        kind: if favorite.is_favorite() {
            StarKind::Filled
        } else {
            StarKind::Outline
        },
        shown: favorite.is_favorite(),
    }
}

fn row_name(entry: &ModelEntry, kind: SectionKind, selected: Selected, state: &State) -> String {
    let mut name = visible_name(entry, kind);
    if selected.is_selected() && !entry.variants.is_empty() {
        let variant = state
            .current_selection()
            .and_then(|selection| selection.variant())
            .and_then(|choice| choice.variant())
            .unwrap_or("default");
        name.push(' ');
        name.push_str(variant);
    }
    name
}

/// The model's visible label: name, provider when the group shows it, and context.
pub(super) fn visible_name(entry: &ModelEntry, kind: SectionKind) -> String {
    let mut name = crate::views::model_names::model_display_name(&entry.name, &entry.model_id);
    if kind.shows_provider() {
        name.push(' ');
        name.push_str(&entry.provider_name);
    }
    if let Some(context) = entry.context {
        name.push(' ');
        name.push_str(&compact_tokens(context));
    }
    name
}

#[cfg(test)]
#[path = "model_rows_test.rs"]
mod model_rows_test;
