//! The four picker popups, placed from their trigger bounds and the CSS React anchors them
//! with.
//!
//! The panels are laid out by `Anchor` rules ported from Base UI's positioner configuration
//! (`DraftTargetSelectors.tsx:363-370`, `ModelControls.tsx:2407-2413,2747,2894`): a popup
//! below the project trigger, and the three composer menus above it, right-aligned by 40px.

mod agent;
mod common;
mod model;
mod project;
mod thinking;

use gpui_kit::{Anchor, AnyElement, App, IntoElement, ParentElement, Window, anchored, point, px};
use oc_state::{Project, State, model_sections, ranked_agents, ranked_projects};

use super::Scene;

pub(crate) use types::{Pick, PopoverKind};

mod types;

/// The popup positioned from the trigger it belongs to.
pub(crate) fn popover(scene: Scene<'_>, window: &mut Window, cx: &App) -> Option<AnyElement> {
    let kind = scene.popover?;
    let trigger = scene.triggers.get(kind)?;
    let viewport = window.viewport_size().height;
    let content_origin = scene
        .registry
        .last_content()
        .map_or(0., |bounds| f32::from(bounds.origin.y));
    let available = available_height(trigger, viewport, content_origin);
    let panel = match kind {
        PopoverKind::Project => project::panel(scene, cx),
        PopoverKind::Model => model::panel(scene, cx, available),
        PopoverKind::Agent => agent::panel(scene, cx),
        PopoverKind::Thinking => thinking::panel(scene, window, cx),
    };
    let mut placement = anchored().position_mode(gpui_kit::AnchoredPositionMode::Window);
    placement = match kind {
        // Side bottom, align start, sideOffset 4 (`DraftTargetSelectors.tsx:363-368`).
        PopoverKind::Project => placement
            .attach(trigger, Anchor::BottomLeft)
            .anchor(Anchor::TopLeft)
            .offset(point(px(2.), px(4.))),
        // Side top, align end, alignOffset -40, sideOffset 4 (`dropdown-menu.tsx:155,205-212`).
        // The fixed-size menus are placed from their own box so GPUI's rounded right edge does
        // not move the panel when its width changes.
        PopoverKind::Model => {
            let left = trigger.right() + px(40.) - px(model::WIDTH);
            let top = trigger.top() - px(4.) - px(available);
            placement.position(point(left, top))
        }
        PopoverKind::Thinking => {
            let left = trigger.right() + px(40.) - px(thinking::panel_width(window, cx));
            let top = trigger.top() - px(4.) - px(thinking::panel_height(scene));
            placement.position(point(left, top))
        }
        PopoverKind::Agent => placement
            .attach(trigger, Anchor::TopRight)
            .anchor(Anchor::BottomRight)
            .offset(point(px(40.), px(-4.))),
    };
    Some(placement.child(panel).into_any_element())
}

/// The rows Enter can pick, in the popup's drawn order.
pub(crate) fn selectables(state: &State, kind: PopoverKind, query: &str) -> Vec<Pick> {
    match kind {
        PopoverKind::Project => {
            let Some(settings) = state.settings() else {
                return Vec::new();
            };
            std::iter::once(Pick::Chat)
                .chain(
                    ranked_projects(settings, query)
                        .into_iter()
                        .filter_map(project_id),
                )
                .collect()
        }
        PopoverKind::Model => model_rows(state, query),
        PopoverKind::Agent => ranked_agents_in(state, query)
            .into_iter()
            .map(|agent| Pick::Agent(agent.id.clone()))
            .collect(),
        PopoverKind::Thinking => {
            let mut picks = vec![Pick::Variant(None)];
            picks.extend(
                selected_model_variants(state)
                    .into_iter()
                    .map(|variant| Pick::Variant(Some(variant))),
            );
            picks
        }
    }
}

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

/// The catalog models on a row list, in the popup's order.
pub(crate) fn model_rows(state: &State, query: &str) -> Vec<Pick> {
    let (Some(settings), Some(directory)) = (state.settings(), selection_directory(state)) else {
        return Vec::new();
    };
    let Some(catalog) = state.catalog(&directory) else {
        return Vec::new();
    };
    let sections = model_sections(catalog, settings, query);
    sections
        .favorites
        .iter()
        .chain(sections.recent.iter())
        .chain(
            sections
                .providers
                .iter()
                .flat_map(|section| section.models.iter()),
        )
        .map(model_pick)
        .collect()
}

/// The agent rows the menu draws for `query`, ranked as React ranks them.
pub(crate) fn ranked_agents_in<'a>(
    state: &'a State,
    query: &str,
) -> Vec<&'a oc_client::catalog::Agent> {
    let Some(directory) = selection_directory(state) else {
        return Vec::new();
    };
    let Some(catalog) = state.catalog(&directory) else {
        return Vec::new();
    };
    ranked_agents(catalog, query)
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
    let name = super::model_names::model_display_name(&model.name, &model.id);
    Some(format!("{provider_id} logo {name}"))
}

/// The agent trigger's label: the display name, else the id capitalized.
pub(crate) fn selected_agent_label(state: &State) -> Option<String> {
    let agent = state.current_selection()?.agent()?;
    let directory = selection_directory(state)?;
    let catalog = state.catalog(&directory)?;
    let entry = catalog.agent(agent)?;
    Some(super::model_names::agent_display_name(
        &entry.display_name,
        &entry.id,
    ))
}

/// One model row's pick.
pub(crate) fn model_pick(entry: &oc_state::ModelEntry) -> Pick {
    Pick::Model {
        provider_id: entry.provider_id.clone(),
        model_id: entry.model_id.clone(),
    }
}

/// The space above the trigger that Base UI gives the model popup: the chat area above the
/// trigger minus the side offset (4) and Base UI's 5px collision padding.
fn available_height(
    trigger: gpui_kit::Bounds<gpui_kit::Pixels>,
    viewport: gpui_kit::Pixels,
    content_origin: f32,
) -> f32 {
    const CHAT_TOP: f32 = 48.;
    const SIDE_OFFSET: f32 = 4.;
    const COLLISION_PADDING: f32 = 5.;
    let top = f32::from(trigger.origin.y) - content_origin;
    let bottom = f32::from(trigger.bottom()) - content_origin;
    let above = top - CHAT_TOP - SIDE_OFFSET - COLLISION_PADDING;
    let below = f32::from(viewport) - content_origin - bottom - SIDE_OFFSET - COLLISION_PADDING;
    if above >= 200. {
        above.min(400.)
    } else {
        below.min(400.)
    }
}

fn project_id(project: &Project) -> Option<Pick> {
    project.id().map(|id| Pick::Project(id.to_owned()))
}

#[cfg(test)]
#[path = "popover_test.rs"]
mod popover_test;
