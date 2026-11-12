//! Auto-accept modes, held-back fresh requests, and the safety net's holds.
//!
//! The server owns the policy. A newer snapshot replaces the modes; a session inherits the
//! nearest explicit mode up its lineage, and unknown lineage is `ask`
//! (`packages/web/server/lib/permission-auto-accept/runtime.js:224-255`,
//! `stores/utils/permissionAutoAccept.ts:80-115`). A fresh `permission.asked` the server may
//! answer waits outside the store until `left-for-user` names it or a reply drops it
//! (`sync/sync-context.tsx:1929-1990`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use oc_client::permission::PermissionRequest;
use oc_client::routing::{
    AutoAcceptSnapshot, PermissionMode, RoutingPermissionHeld, RoutingState, RoutingUpdated,
};

use crate::permission_mode::{SafetyNet, displayed_permission_mode, next_permission_mode};
use crate::safety_hold::SafetyHold;
use crate::session_entry::PendingRequest;
use crate::state::State;

/// React's cap on reports that arrive before their request (`sync-context.tsx:1931`).
const MAX_EARLY_LEFT_FOR_USER: usize = 100;

/// Modes, routing availability, held-back requests and the safety net's holds.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AutoAccept {
    modes: BTreeMap<String, PermissionMode>,
    revision: Option<u64>,
    routing_available: bool,
    safety_usable: bool,
    held: BTreeMap<String, SafetyHold>,
    awaiting: BTreeMap<(String, String), PermissionRequest>,
    early_left_for_user: BTreeSet<(String, String)>,
    early_order: VecDeque<(String, String)>,
}

impl AutoAccept {
    /// Applies a policy snapshot; an older revision keeps the modes already held.
    pub(crate) fn apply_snapshot(&mut self, snapshot: AutoAcceptSnapshot) -> bool {
        if let Some(held) = self.revision
            && snapshot.revision < held
        {
            return false;
        }
        self.modes = snapshot.modes;
        self.revision = Some(snapshot.revision);
        true
    }

    /// Applies `GET /api/routing`, replacing the held requests with the snapshot's.
    pub(crate) fn apply_routing(&mut self, routing: RoutingState) {
        self.routing_available = routing.available;
        self.safety_usable = routing.jev_available;
        self.held = routing
            .held_permissions
            .iter()
            .map(|held| (held.permission_id.clone(), SafetyHold::from_routing(held)))
            .collect();
    }

    /// Applies `routing.updated`: the capabilities moved, the holds did not.
    pub(crate) fn apply_routing_updated(&mut self, updated: &RoutingUpdated) {
        self.routing_available = updated.available;
        self.safety_usable = updated.jev_available;
    }

    /// The safety net can classify an action (`useRoutingStore.ts:160`).
    fn safety_net(&self) -> SafetyNet {
        if self.routing_available && self.safety_usable {
            SafetyNet::Available
        } else {
            SafetyNet::Unavailable
        }
    }

    /// The nearest explicit mode up the lineage, `ask` when there is none.
    fn resolve(&self, session: &str, parent_of: impl Fn(&str) -> Option<String>) -> PermissionMode {
        if self.modes.is_empty() {
            return PermissionMode::Ask;
        }
        let mut current = Some(session.to_owned());
        let mut seen = BTreeSet::new();
        while let Some(id) = current {
            if !seen.insert(id.clone()) {
                break;
            }
            if let Some(mode) = self.modes.get(&id) {
                return *mode;
            }
            current = parent_of(&id);
        }
        PermissionMode::Ask
    }

    fn is_answered_without_user(&self, mode: PermissionMode) -> bool {
        mode == PermissionMode::Auto
            || (mode == PermissionMode::Safety && self.safety_net().available())
    }

    fn hold(&mut self, held: &RoutingPermissionHeld) {
        self.held
            .insert(held.permission_id.clone(), SafetyHold::from_wire(held));
    }

    fn release(&mut self, permission_id: &str) {
        self.held.remove(permission_id);
    }

    fn take_early(&mut self, key: &(String, String)) -> bool {
        if !self.early_left_for_user.remove(key) {
            return false;
        }
        self.early_order.retain(|held| held != key);
        true
    }

    fn remember_early(&mut self, key: (String, String)) {
        if !self.early_left_for_user.insert(key.clone()) {
            return;
        }
        self.early_order.push_back(key);
        while self.early_order.len() > MAX_EARLY_LEFT_FOR_USER {
            if let Some(oldest) = self.early_order.pop_front() {
                self.early_left_for_user.remove(&oldest);
            }
        }
    }

    fn forget(&mut self, session: &str, permission_id: &str) {
        let key = (session.to_owned(), permission_id.to_owned());
        self.awaiting.remove(&key);
        self.take_early(&key);
    }
}

impl State {
    /// Whether the safety net can classify an action.
    pub fn safety_net(&self) -> SafetyNet {
        self.auto_accept.safety_net()
    }

    /// The session's effective mode, through its nearest explicit ancestor.
    pub fn effective_mode(&self, session: &str) -> PermissionMode {
        self.auto_accept.resolve(session, |id| self.parent_of(id))
    }

    /// The mode the shield shows.
    pub fn displayed_mode(&self, session: &str) -> PermissionMode {
        displayed_permission_mode(self.effective_mode(session), self.safety_net())
    }

    /// The next mode a shield click asks the server for.
    pub fn next_mode(&self, session: &str) -> PermissionMode {
        next_permission_mode(self.effective_mode(session), self.safety_net())
    }

    /// The safety net's hold on a request, when it holds one.
    pub fn held_permission(&self, permission_id: &str) -> Option<&SafetyHold> {
        self.auto_accept.held.get(permission_id)
    }

    /// Whether the server may answer this session's fresh requests without the user.
    pub(crate) fn answered_without_user(&self, session: &str) -> bool {
        self.auto_accept
            .is_answered_without_user(self.effective_mode(session))
    }

    /// Applies a fresh `permission.asked`: held back when the server may answer it, shown
    /// when a `left-for-user` report already named it (`sync-context.tsx:1955-1968`).
    pub(crate) fn add_fresh_permission(&mut self, request: PermissionRequest) {
        let key = (request.session_id.clone(), request.id.clone());
        let directory = self
            .session_directory(&request.session_id)
            .unwrap_or_else(|| self.directory.clone());
        if self.auto_accept.take_early(&key) {
            self.requests.upsert_permission(request, &directory);
            self.reflect_requests();
            return;
        }
        if self.answered_without_user(&request.session_id) {
            self.auto_accept.awaiting.insert(key, request);
            return;
        }
        self.requests.upsert_permission(request, &directory);
        self.reflect_requests();
    }

    /// Applies `left-for-user`: shows the waiting request, or remembers the report for a
    /// request that has not arrived yet.
    pub(crate) fn permission_left_for_user(&mut self, session: &str, permission_id: &str) {
        let key = (session.to_owned(), permission_id.to_owned());
        if let Some(request) = self.auto_accept.awaiting.remove(&key) {
            let directory = self
                .session_directory(&request.session_id)
                .unwrap_or_else(|| self.directory.clone());
            self.requests.upsert_permission(request, &directory);
            self.reflect_requests();
            return;
        }
        if self.requests.has_permission(session, permission_id) {
            return;
        }
        self.auto_accept.remember_early(key);
    }

    /// Settles a request: drops the waiting entry, any early report, the safety hold and
    /// the shown card.
    pub(crate) fn settle_permission(&mut self, session: &str, permission_id: &str) {
        self.auto_accept.forget(session, permission_id);
        self.auto_accept.release(permission_id);
        self.requests.remove_permission(session, permission_id);
        self.reflect_requests();
    }

    /// Adds a safety net hold reported by `routing.permission-held`.
    pub(crate) fn hold_permission(&mut self, held: &RoutingPermissionHeld) {
        self.auto_accept.hold(held);
    }

    /// Rebuilds every listed row's pending marks from the requests store.
    pub(crate) fn reflect_requests(&mut self) {
        let mut pending: BTreeMap<String, Vec<PendingRequest>> = BTreeMap::new();
        for entry in self.sessions.iter().chain(self.global_sessions().iter()) {
            pending
                .entry(entry.id.as_str().to_owned())
                .or_insert_with(|| self.requests.pending_for(entry.id.as_str()));
        }
        for (id, marks) in pending {
            for list in [&mut self.sessions, &mut self.global.list] {
                if let Some(entry) = list.get_mut(&id) {
                    entry.pending = marks.clone();
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "auto_accept_test.rs"]
mod auto_accept_test;
