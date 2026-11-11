//! Pending permission requests (`GET /api/permission/request`, `permission.asked`).
//!
//! Shapes follow the OpenCode contract (`@opencode/client` generated `PermissionRequest`).
//! `action` and `resources` carry a default because `oc-state`'s hand-built
//! `permission.asked` payloads name only the request id; the captured server always sends
//! them. Unknown fields are ignored.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One pending request: a list entry and the `permission.asked` payload.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PermissionRequest {
    /// `per_…` id.
    pub id: String,
    /// The asking session.
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// The permission action: `shell` for bash, `edit` for edit and write.
    #[serde(default)]
    pub action: String,
    /// What the tool asks to touch; for a shell action, the command lines.
    #[serde(default)]
    pub resources: Vec<String>,
    /// Patterns an `always` decision saves; empty when the tool saves nothing.
    #[serde(default)]
    pub save: Vec<String>,
    /// Tool metadata the preview reads, such as `files` or `command`; absent on a
    /// request without one.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    /// The tool call that asked.
    #[serde(default)]
    pub source: Option<PermissionSource>,
    /// A message the server adds to the request.
    #[serde(default)]
    pub message: Option<String>,
}

/// The tool call behind a [`PermissionRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PermissionSource {
    /// What raised the request.
    #[serde(rename = "type")]
    pub kind: PermissionSourceKind,
    /// The assistant message that called the tool.
    #[serde(rename = "messageID")]
    pub message_id: String,
    /// The tool call id.
    pub id: String,
}

/// What raised a [`PermissionRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionSourceKind {
    /// A tool call.
    Tool,
    /// A source this client does not know.
    #[serde(other)]
    Other,
}

/// The answer to a [`PermissionRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionReply {
    /// Allow this request only.
    Once,
    /// Allow and save the request's patterns.
    Always,
    /// Deny the request.
    Reject,
}

/// Body of `POST /api/session/:id/permission/:requestId/reply`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PermissionReplyRequest {
    /// The answer to send.
    pub decision: PermissionReply,
}

#[cfg(test)]
#[path = "permission_test.rs"]
mod permission_test;
