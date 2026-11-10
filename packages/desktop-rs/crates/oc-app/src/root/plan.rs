//! What the draft would send with, and where its session goes.

use oc_state::{DraftTarget, ModelRef, Selection, State};

use crate::live::{CreateTarget, new_chat_folder};

/// The agent and model a selection would send with.
pub(super) fn selection_parts(selection: Option<&Selection>) -> (Option<String>, Option<ModelRef>) {
    let Some(selection) = selection else {
        return (None, None);
    };
    let agent = selection.agent().map(str::to_owned);
    let model = selection
        .model()
        .map(|key| key.to_model_ref(selection.variant().and_then(|choice| choice.variant())));
    (agent, model)
}

/// Where a draft's send goes: its project directory, or a freshly minted Chat folder.
pub(super) fn create_target(state: &State) -> Option<CreateTarget> {
    match &state.draft().target {
        DraftTarget::Project { directory, .. } => Some(CreateTarget::Project {
            directory: directory.clone(),
        }),
        DraftTarget::Chat => state
            .home()
            .and_then(new_chat_folder)
            .map(|folder| CreateTarget::Chat { folder }),
    }
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
