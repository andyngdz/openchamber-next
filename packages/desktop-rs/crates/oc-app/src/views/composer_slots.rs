//! The composer footer's slot list, in reference order.

use gpui_kit::assets::IconName;
use oc_state::{LoadStatus, SessionStatus, State};

use crate::views::popover::{
    DEFAULT_MODEL, PopoverKind, selected_agent_label, selected_model_label, selection_directory,
};

const AGENT: &str = "Build";
const UNAVAILABLE: &str = "Unavailable";

/// Which trigger layout a footer slot uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TriggerKind {
    Model,
    Thinking,
    Agent,
}

/// One footer slot, left to right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Tool {
    Icon(&'static str, IconName),
    Trigger {
        kind: PopoverKind,
        layout: TriggerKind,
        name: String,
        label: String,
    },
    Send,
    Stop,
}

/// The footer slots in reference order.
pub(super) fn toolbar(state: &State) -> Vec<Tool> {
    let mut tools = vec![
        Tool::Icon("Add attachment", IconName::Plus),
        Tool::Icon("Permissions: ask every time", IconName::CircleCheck),
        Tool::Icon("Start a goal with the next message", IconName::Star),
        model_tool(state),
    ];
    if let Some(tool) = thinking_tool(state) {
        tools.push(tool);
    }
    tools.push(agent_tool(state));
    tools.push(Tool::Icon("Start dictation", IconName::Mic));
    tools.push(match state.open_status() {
        SessionStatus::Busy => Tool::Stop,
        SessionStatus::Idle | SessionStatus::Error => Tool::Send,
    });
    tools
}

/// The name of a slot, in reference order.
pub(super) fn tool_name(tool: &Tool) -> Option<String> {
    match tool {
        Tool::Icon(name, _) => Some((*name).to_owned()),
        Tool::Trigger { name, .. } => Some(name.clone()),
        Tool::Send => Some("Send message".to_owned()),
        Tool::Stop => Some("Stop generating".to_owned()),
    }
}

fn model_tool(state: &State) -> Tool {
    let unavailable = selection_directory(state)
        .as_deref()
        .and_then(|directory| state.catalog_load(directory))
        .is_some_and(|load| matches!(load, LoadStatus::Failed { .. }));
    if unavailable {
        return Tool::Trigger {
            kind: PopoverKind::Model,
            layout: TriggerKind::Model,
            name: UNAVAILABLE.to_owned(),
            label: UNAVAILABLE.to_owned(),
        };
    }
    let name = selected_model_label(state).unwrap_or_else(|| DEFAULT_MODEL.to_owned());
    let label = name
        .split_once(" logo ")
        .map_or_else(|| name.clone(), |(_, model)| model.to_owned());
    Tool::Trigger {
        kind: PopoverKind::Model,
        layout: TriggerKind::Model,
        name,
        label,
    }
}

fn thinking_tool(state: &State) -> Option<Tool> {
    if crate::views::popover::selected_model_variants(state).is_empty() {
        return None;
    }
    let variant = state
        .current_selection()
        .and_then(|selection| selection.variant())
        .and_then(|choice| choice.variant());
    let label = crate::views::model_names::effort_label(variant);
    let name = if state.open().is_some() {
        format!("Thinking: {label}")
    } else {
        format!("Thinking: {label}, last used with this model")
    };
    Some(Tool::Trigger {
        kind: PopoverKind::Thinking,
        layout: TriggerKind::Thinking,
        name,
        label: label.to_lowercase(),
    })
}

fn agent_tool(state: &State) -> Tool {
    let label = selected_agent_label(state).unwrap_or_else(|| AGENT.to_owned());
    Tool::Trigger {
        kind: PopoverKind::Agent,
        layout: TriggerKind::Agent,
        name: label.clone(),
        label,
    }
}

#[cfg(test)]
#[path = "composer_slots_test.rs"]
mod composer_slots_test;
