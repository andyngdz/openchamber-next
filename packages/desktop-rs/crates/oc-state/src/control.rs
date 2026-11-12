//! The `/api/openchamber/events` control frames this state applies
//! (`oc_client::control`; design "Control stream").
//!
//! A ready frame asks the app to re-read its snapshots; the queue belongs to the queue
//! state. This module applies the policy, the left-for-user reports and the safety holds.

use oc_client::control::ControlEvent;

use crate::state::State;

/// Applies one control frame.
pub(crate) fn reduce_control(state: &mut State, event: &ControlEvent) {
    match event {
        ControlEvent::AutoAcceptUpdated(snapshot) => {
            if !state.auto_accept.apply_snapshot(snapshot.clone()) {
                tracing::debug!(
                    target: crate::logging::STATE,
                    revision = snapshot.revision,
                    "[STATE] Ignored an older auto-accept snapshot"
                );
            }
        }
        ControlEvent::LeftForUser(left) => {
            state.permission_left_for_user(&left.session_id, &left.permission_id);
        }
        ControlEvent::RoutingUpdated(updated) => {
            state.auto_accept.apply_routing_updated(updated);
        }
        ControlEvent::PermissionHeld(held) => state.hold_permission(held),
        ControlEvent::Ready | ControlEvent::Heartbeat | ControlEvent::Other { .. } => {}
        ControlEvent::MessageQueueUpdated(_) => {}
    }
}

#[cfg(test)]
#[path = "control_test.rs"]
mod control_test;
