//! State changes the turn-control calls report, applied by the app after its own requests.
//!
//! These are separate from [`crate::Action`] because the app's existing action match is
//! closed; the live wiring adds a channel for them.

use oc_client::control::ControlEvent;
use oc_client::form::FormInfo;
use oc_client::permission::PermissionRequest;
use oc_client::routing::{AutoAcceptSnapshot, RoutingState};

use crate::control::reduce_control;
use crate::state::State;

impl State {
    /// `GET /api/permission/request` answered for a directory; only that directory's
    /// entries are replaced.
    pub fn permissions_loaded(&mut self, directory: &str, requests: Vec<PermissionRequest>) {
        self.requests.replace_permissions(directory, requests);
        self.reflect_requests();
    }

    /// The permission list failed; every previous entry stays and the directory is stale.
    pub fn permissions_failed(&mut self, directory: &str, message: &str) {
        self.requests.fail_permissions(directory);
        tracing::debug!(
            target: crate::logging::STATE,
            directory,
            error = %message,
            "[STATE] Permission list kept its previous entries"
        );
    }

    /// `GET /api/form` answered for a directory, `global` forms included.
    pub fn forms_loaded(&mut self, directory: &str, forms: Vec<FormInfo>) {
        self.recoverable.note_snapshot(directory, &forms);
        self.requests.replace_forms(directory, forms);
        self.reflect_requests();
    }

    /// The form list failed; every previous entry stays and the directory is stale.
    pub fn forms_failed(&mut self, directory: &str, message: &str) {
        self.requests.fail_forms(directory);
        tracing::debug!(
            target: crate::logging::STATE,
            directory,
            error = %message,
            "[STATE] Form list kept its previous entries"
        );
    }

    /// `GET /api/permission-auto-accept` answered; an older revision changes nothing.
    pub fn auto_accept_loaded(&mut self, snapshot: AutoAcceptSnapshot) {
        self.auto_accept.apply_snapshot(snapshot);
    }

    /// The auto-accept read failed; the modes already held stay.
    pub fn auto_accept_failed(&mut self, message: &str) {
        tracing::debug!(
            target: crate::logging::STATE,
            error = %message,
            "[STATE] Auto-accept read kept the modes already held"
        );
    }

    /// `GET /api/routing` answered; its held requests become the safety holds.
    pub fn routing_loaded(&mut self, routing: RoutingState) {
        self.auto_accept.apply_routing(routing);
    }

    /// The routing read failed; what this server already reported stays.
    pub fn routing_failed(&mut self, message: &str) {
        tracing::debug!(
            target: crate::logging::STATE,
            error = %message,
            "[STATE] Routing read kept the state already held"
        );
    }

    /// One frame of `/api/openchamber/events`.
    pub fn control_event(&mut self, event: &ControlEvent) {
        reduce_control(self, event);
    }

    /// The user's answer to a question was accepted.
    pub fn form_answered(&mut self, form_id: &str) {
        self.recoverable.note_replied(form_id);
    }

    /// The user dismissed a form or recovered question.
    pub fn form_dismissed(&mut self, form_id: &str) {
        self.recoverable.note_cancelled(form_id);
    }

    /// Whether the directory's pending lists failed and wait for the next reconnect.
    pub fn requests_stale(&self, directory: &str) -> bool {
        self.requests.is_stale(directory)
    }
}

#[cfg(test)]
#[path = "turn_actions_test.rs"]
mod turn_actions_test;
