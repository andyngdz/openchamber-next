//! Events of `GET /api/openchamber/events`, the server's own control stream.
//!
//! Frames are `data:` lines only, each a flat `{type, properties}` with no event envelope
//! (`packages/web/server/lib/notifications/emitter-runtime.js:26-28`). Every type this app
//! does not act on decodes to [`ControlEvent::Other`] and is dropped, never an error.

use serde::Deserialize;
use serde_json::Value;

use crate::queue::QueueUpdate;
use crate::routing::{AutoAcceptSnapshot, RoutingPermissionHeld, RoutingUpdated};

/// One decoded control frame.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawControlEvent")]
pub enum ControlEvent {
    /// `openchamber:event-stream-ready`: the stream opened; re-read the snapshots.
    Ready,
    /// `openchamber:heartbeat`: the stream is alive; says nothing else.
    Heartbeat,
    /// `openchamber:message-queue.updated`: one session's queue changed.
    MessageQueueUpdated(QueueUpdate),
    /// `openchamber:permission-auto-accept.updated`: the whole policy moved.
    AutoAcceptUpdated(AutoAcceptSnapshot),
    /// `openchamber:permission-auto-accept.left-for-user`: a request needs the user.
    LeftForUser(LeftForUser),
    /// `openchamber:routing.updated`: the routing capabilities moved.
    RoutingUpdated(RoutingUpdated),
    /// `openchamber:routing.permission-held`: the safety net held a request.
    PermissionHeld(RoutingPermissionHeld),
    /// A type this app does not act on, dropped with its properties.
    Other {
        /// The wire type.
        kind: String,
    },
}

/// `openchamber:permission-auto-accept.left-for-user`: the server stopped answering a
/// request and left it for the user.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LeftForUser {
    /// The request that needs the user.
    #[serde(rename = "permissionId")]
    pub permission_id: String,
    /// The asking session.
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Where the request lives, when the server knows.
    #[serde(default)]
    pub directory: Option<String>,
}

#[derive(Deserialize)]
struct RawControlEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    properties: Value,
}

impl TryFrom<RawControlEvent> for ControlEvent {
    type Error = serde_json::Error;

    fn try_from(raw: RawControlEvent) -> Result<Self, Self::Error> {
        let RawControlEvent { kind, properties } = raw;
        Ok(match kind.as_str() {
            "openchamber:event-stream-ready" => Self::Ready,
            "openchamber:heartbeat" => Self::Heartbeat,
            "openchamber:message-queue.updated" => {
                Self::MessageQueueUpdated(serde_json::from_value(properties)?)
            }
            "openchamber:permission-auto-accept.updated" => {
                Self::AutoAcceptUpdated(serde_json::from_value(properties)?)
            }
            "openchamber:permission-auto-accept.left-for-user" => {
                Self::LeftForUser(serde_json::from_value(properties)?)
            }
            "openchamber:routing.updated" => {
                Self::RoutingUpdated(serde_json::from_value(properties)?)
            }
            "openchamber:routing.permission-held" => {
                Self::PermissionHeld(serde_json::from_value(properties)?)
            }
            _ => Self::Other { kind },
        })
    }
}

#[cfg(test)]
#[path = "control_test.rs"]
mod control_test;
