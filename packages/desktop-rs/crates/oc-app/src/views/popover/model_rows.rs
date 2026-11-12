//! The model picker's rows and sections.
//!
//! Every section carries a header: `List` measures one header size, from the first section,
//! and lays out every section with it, so a section without a header would draw every header
//! at zero height, over the row below it.

use oc_state::{ModelEntry, ModelSections, State, model_sections};

use super::list::RowAction;
use super::list::{Header, Row, Section, Star, StarKind};
use crate::icons::RemixIcon;
use crate::layout::Role;
use crate::views::model_names::{compact_tokens, model_display_name};

pub(super) const MODELS_GROUP: &str = "Models";
pub(super) const ADD_PROVIDER: &str = "Add new provider";
pub(super) const RUN_ON_SEVERAL: &str = "Run on several models";
const FAVORITES: &str = "Favorites";
const RECENT: &str = "Recent";
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

/// Which group a row belongs to, which decides whether its accessible name and row show the
/// provider.
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

    fn header_icon(self) -> Option<RemixIcon> {
        match self {
            Self::Favorites => Some(RemixIcon::StarFill),
            Self::Recent => Some(RemixIcon::Time),
            Self::Provider => None,
        }
    }
}

/// The panel's sections in drawn order, filtered by `query`.
pub(super) fn sections(state: &State, query: &str) -> Vec<Section> {
    let Some(found) = found_sections(state, query) else {
        return Vec::new();
    };
    let mut sections = vec![
        group(
            "FAVORITES",
            FAVORITES,
            &found.favorites,
            SectionKind::Favorites,
            state,
        ),
        group("RECENT", RECENT, &found.recent, SectionKind::Recent, state),
    ];
    sections.extend(
        found
            .providers
            .iter()
            .map(|provider| provider_section(provider, state)),
    );
    sections
}

/// The action pinned above the sections, which React keeps out of the list.
pub(super) fn leading_row() -> Row {
    Row {
        name: RUN_ON_SEVERAL.to_owned(),
        label: RUN_ON_SEVERAL.to_owned(),
        icon: None,
        logo: None,
        dot: None,
        context: None,
        selected: false,
        role: Role::Option,
        ancestors: vec![MODELS_GROUP.to_owned()],
        action: RowAction::Close,
        star: None,
        described: None,
        repeat: None,
        star_under_row: true,
    }
}

fn found_sections(state: &State, query: &str) -> Option<ModelSections> {
    let settings = state.settings()?;
    let directory = super::selection_directory(state)?;
    let catalog = state.catalog(&directory)?;
    Some(model_sections(catalog, settings, query))
}

fn provider_section(provider: &oc_state::ProviderSection, state: &State) -> Section {
    Section {
        header: Some(Header {
            name: format!(
                "{} logo {}",
                provider.provider_id,
                provider.name.to_uppercase()
            ),
            label: provider.name.to_uppercase(),
            icon: None,
            logo: Some(provider.provider_id.clone()),
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
            label: label.to_uppercase(),
            icon: kind.header_icon(),
            logo: None,
            ancestors: vec![MODELS_GROUP.to_owned(), label.to_owned()],
        }),
        rows: entries
            .iter()
            .map(|entry| model_row(entry, label, kind, state))
            .collect(),
    }
}

fn model_row(entry: &ModelEntry, group: &str, kind: SectionKind, state: &State) -> Row {
    let selected = if is_selected(state, entry) {
        Selected::Yes
    } else {
        Selected::No
    };
    Row {
        name: accessible_name(entry, kind, selected, state),
        label: model_display_name(&entry.name, &entry.model_id),
        icon: None,
        logo: kind.shows_provider().then(|| entry.provider_id.clone()),
        dot: None,
        context: entry.context.map(compact_tokens),
        selected: selected.is_selected(),
        role: Role::Option,
        ancestors: vec![MODELS_GROUP.to_owned(), group.to_owned()],
        action: RowAction::Pick(super::Pick::Model {
            provider_id: entry.provider_id.clone(),
            model_id: entry.model_id.clone(),
        }),
        star: Some(model_star(state, entry)),
        described: None,
        repeat: None,
        star_under_row: true,
    }
}

fn is_selected(state: &State, entry: &ModelEntry) -> bool {
    state
        .current_selection()
        .and_then(|selection| selection.model())
        .is_some_and(|key| key.provider_id() == entry.provider_id && key.id() == entry.model_id)
}

fn model_star(state: &State, entry: &ModelEntry) -> Star {
    let starred = state.settings().is_some_and(|settings| {
        settings.favorite_models.iter().any(|reference| {
            reference.provider_id == entry.provider_id && reference.model_id == entry.model_id
        })
    });
    let kind = if starred {
        StarKind::Filled
    } else {
        StarKind::Outline
    };
    Star::model(entry, kind)
}

fn accessible_name(
    entry: &ModelEntry,
    kind: SectionKind,
    selected: Selected,
    state: &State,
) -> String {
    let mut name = readable_name(entry, kind);
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

/// The model's accessible name: name, provider when the group shows it, and context.
pub(super) fn readable_name(entry: &ModelEntry, kind: SectionKind) -> String {
    let mut name = model_display_name(&entry.name, &entry.model_id);
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
