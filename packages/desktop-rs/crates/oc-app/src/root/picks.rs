//! Turning a popup row into the selection change the live app applies.

use gpui_kit::{Context, Window};
use oc_state::SelectionAction;

use super::RootView;
use crate::views::Pick;

impl RootView {
    /// Applies the picker row under the cursor and closes the popup.
    pub(super) fn pick(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        let action = action_for(pick, chrono::Utc::now().timestamp_millis() as u64);
        self.apply_selection(action, cx);
        self.close_popover(window, cx);
    }
}

fn action_for(pick: Pick, now: u64) -> SelectionAction {
    match pick {
        Pick::Chat => SelectionAction::PickChat,
        Pick::Project(id) => SelectionAction::PickProject { id, now },
        Pick::Model {
            provider_id,
            model_id,
        } => SelectionAction::PickModel {
            provider_id,
            model_id,
        },
        Pick::Agent(agent) => SelectionAction::PickAgent(agent),
        Pick::Variant(variant) => SelectionAction::PickVariant(variant),
    }
}

#[cfg(test)]
#[path = "picks_test.rs"]
mod picks_test;
