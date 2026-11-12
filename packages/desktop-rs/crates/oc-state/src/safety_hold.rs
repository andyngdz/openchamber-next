//! A permission request the safety net stopped for the user to decide.

use oc_client::routing::{HeldPermission, RoutingPermissionHeld};

/// One request the safety net stopped auto-accept for.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyHold {
    /// The held request.
    pub permission_id: String,
    /// The asking session, when the report named it.
    pub session_id: Option<String>,
    /// The action kind the classifier named.
    pub kind: Option<String>,
    /// The classifier's confidence that the action is risky.
    pub score: f64,
}

impl SafetyHold {
    /// A hold from `GET /api/routing`'s `heldPermissions`.
    pub(crate) fn from_routing(held: &HeldPermission) -> Self {
        Self {
            permission_id: held.permission_id.clone(),
            session_id: None,
            kind: held.kind.clone(),
            score: held.score,
        }
    }

    /// A hold from `openchamber:routing.permission-held`.
    pub(crate) fn from_wire(held: &RoutingPermissionHeld) -> Self {
        Self {
            permission_id: held.permission_id.clone(),
            session_id: Some(held.session_id.clone()),
            kind: held.kind.clone(),
            score: held.score,
        }
    }
}
