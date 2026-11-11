//! The permission auto-accept policy and the routing state it depends on.
//!
//! `/api/permission-auto-accept` answers `{sessions, modes, revision}`; `sessions` is the
//! retired on/off map and `modes` names the mode per session. `/api/routing` answers more,
//! of which this app reads `available`, `jevAvailable` and `heldPermissions`; a 404 means
//! the build has no routing at all (`lib/routing/routingApi.ts:119-121`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One session's permission mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    /// Ask the user every time.
    Ask,
    /// Ask only before an action the safety net holds.
    Safety,
    /// Accept every request.
    Auto,
}

/// `GET /api/permission-auto-accept` and `openchamber:permission-auto-accept.updated`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AutoAcceptSnapshot {
    /// The retired on/off map, still sent for clients that know only `enabled`.
    #[serde(default)]
    pub sessions: BTreeMap<String, bool>,
    /// Each session with an explicit mode; a session without one inherits its nearest
    /// ancestor's mode.
    #[serde(default)]
    pub modes: BTreeMap<String, PermissionMode>,
    /// The policy revision; a newer one replaces the whole policy.
    #[serde(default)]
    pub revision: u64,
}

/// Body of `PUT /api/permission-auto-accept/sessions/:id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutoAcceptRequest {
    /// The mode to set.
    pub mode: PermissionMode,
    /// The retired on/off view of the mode, for older servers.
    pub enabled: bool,
    /// The session's directory.
    pub directory: String,
}

/// The routing capabilities this app reads.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RoutingState {
    /// Whether this build has routing at all.
    pub available: bool,
    /// Whether the safety net can classify an action; absent means it always could.
    #[serde(rename = "jevAvailable", default = "safety_always_available")]
    pub jev_available: bool,
    /// The requests the safety net held for the user to decide.
    #[serde(rename = "heldPermissions", default)]
    pub held_permissions: Vec<HeldPermission>,
}

impl RoutingState {
    /// What a 404 reads as: no routing, so no safety net.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            jev_available: false,
            held_permissions: Vec::new(),
        }
    }
}

fn safety_always_available() -> bool {
    true
}

/// One request the safety net held.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HeldPermission {
    /// The held request.
    #[serde(rename = "permissionId")]
    pub permission_id: String,
    /// The classifier's confidence that the action is risky.
    #[serde(default)]
    pub score: f64,
    /// The action kind the classifier named.
    #[serde(default)]
    pub kind: Option<String>,
}

/// `openchamber:routing.updated`: the routing capabilities moved.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RoutingUpdated {
    /// Whether this build has routing at all.
    pub available: bool,
    /// Whether the safety net can classify an action.
    #[serde(rename = "jevAvailable", default = "safety_always_available")]
    pub jev_available: bool,
}

/// `openchamber:routing.permission-held`: the safety net held a request.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RoutingPermissionHeld {
    /// The held request.
    #[serde(rename = "permissionId")]
    pub permission_id: String,
    /// The asking session.
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Where the request lives, when the server knows.
    #[serde(default)]
    pub directory: Option<String>,
    /// The classifier's confidence that the action is risky.
    #[serde(default)]
    pub score: f64,
    /// The action kind the classifier named.
    #[serde(default)]
    pub kind: Option<String>,
}

#[cfg(test)]
#[path = "routing_test.rs"]
mod routing_test;
