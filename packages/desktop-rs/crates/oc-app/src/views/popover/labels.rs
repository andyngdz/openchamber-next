//! Labels and directories the picker triggers and rows read from the shared state.

use oc_state::State;

/// The directory the composer's pickers read the catalog from.
pub(crate) fn selection_directory(state: &State) -> Option<String> {
    if state.open().is_some() {
        return Some(
            state
                .open_entry()
                .map(|entry| entry.directory.clone())
                .unwrap_or_else(|| state.directory().to_owned()),
        );
    }
    match state.draft().target.directory() {
        Some(directory) => Some(directory.to_owned()),
        None => Some(state.directory().to_owned()),
    }
}

/// The selected model's Thinking levels, in the server's order.
pub(crate) fn selected_model_variants(state: &State) -> Vec<String> {
    let Some((provider_id, model_id)) = selected_model(state) else {
        return Vec::new();
    };
    let Some(directory) = selection_directory(state) else {
        return Vec::new();
    };
    let Some(catalog) = state.catalog(&directory) else {
        return Vec::new();
    };
    catalog
        .provider(&provider_id)
        .and_then(|provider| provider.models.iter().find(|model| model.id == model_id))
        .map(|model| {
            model
                .variants
                .iter()
                .map(|variant| variant.id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The selected model's provider and catalog id.
pub(crate) fn selected_model(state: &State) -> Option<(String, String)> {
    let key = state.current_selection()?.model()?;
    Some((key.provider_id().to_owned(), key.id().to_owned()))
}

/// The selected model's label (`provider logo name`).
pub(crate) fn selected_model_label(state: &State) -> Option<String> {
    let (provider_id, model_id) = selected_model(state)?;
    let directory = selection_directory(state)?;
    let catalog = state.catalog(&directory)?;
    let provider = catalog.provider(&provider_id)?;
    let model = provider.models.iter().find(|model| model.id == model_id)?;
    let name = crate::views::model_names::model_display_name(&model.name, &model.id);
    Some(format!("{provider_id} logo {name}"))
}

/// The agent trigger's label: the display name, else the id capitalized.
pub(crate) fn selected_agent_label(state: &State) -> Option<String> {
    let agent = state.current_selection()?.agent()?;
    let directory = selection_directory(state)?;
    let catalog = state.catalog(&directory)?;
    let entry = catalog.agent(agent)?;
    Some(crate::views::model_names::agent_display_name(
        &entry.display_name,
        &entry.id,
    ))
}

/// The agent React's composer shows when the session picked none.
const DEFAULT_AGENT: &str = "Build";

/// The agent the composer's agent trigger shows: the selected one, or the default.
pub(crate) fn shown_agent(state: &State) -> String {
    selected_agent_label(state).unwrap_or_else(|| DEFAULT_AGENT.to_owned())
}

/// The id of the agent the composer's agent trigger shows; empty when the session picked none,
/// which `agent_color` paints as the default `build` agent.
pub(crate) fn shown_agent_id(state: &State) -> String {
    state
        .current_selection()
        .and_then(|selection| selection.agent())
        .map(ToString::to_string)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "labels_test.rs"]
mod labels_test;
