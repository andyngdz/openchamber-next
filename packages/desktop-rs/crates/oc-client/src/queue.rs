//! The server-owned message queue (`/api/message-queue`).
//!
//! Shaped after `packages/web/server/lib/message-queue/runtime.js`: a full `GET` carries
//! `{revision, sessions}`, and one session's snapshot is `{revision, session}` with the
//! public item shape `toPublicItem` builds. Fields the strip does not show are kept, and
//! unknown fields are ignored.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `GET /api/message-queue`: every session's queue at the server's revision.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MessageQueue {
    /// The server's queue revision.
    pub revision: u64,
    /// One entry per session that holds items.
    #[serde(default)]
    pub sessions: Vec<QueueSession>,
}

/// One session's queue.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QueueSession {
    /// The session the items belong to.
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// The session's directory; kept so the UI can key its projection.
    #[serde(default)]
    pub directory: String,
    /// The queued items, oldest first.
    #[serde(default)]
    pub items: Vec<QueueItem>,
    /// The item the server is sending right now, if any.
    #[serde(rename = "sendingId", default)]
    pub sending_id: Option<String>,
}

/// One queued message as the server publishes it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QueueItem {
    /// `queued-…` id.
    pub id: String,
    /// When the item was queued, in milliseconds since the epoch.
    #[serde(rename = "createdAt", default)]
    pub created_at: u64,
    /// The model-facing content.
    #[serde(default)]
    pub content: String,
    /// The text the strip shows.
    #[serde(default)]
    pub text: String,
    /// Attachments, without their payloads; kept but not shown.
    #[serde(default)]
    pub attachments: Vec<Value>,
    /// The first line of any attached context; kept but not shown.
    #[serde(rename = "contextPreview", default)]
    pub context_preview: Option<String>,
    /// The model, agent and variant the server sends the item with.
    #[serde(rename = "sendConfig", default)]
    pub send_config: SendConfig,
    /// Set when a scheduled task queued the item; kept but not shown beyond the row.
    #[serde(rename = "scheduledTask", default)]
    pub scheduled_task: Option<ScheduledTask>,
    /// A "run as goal" record; kept but not shown.
    #[serde(default)]
    pub goal: Option<Value>,
    /// An `@` mention the composer attached; kept but not shown.
    #[serde(rename = "agentMention", default)]
    pub agent_mention: Option<String>,
}

/// How the server sends a queued item.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct SendConfig {
    /// Provider id.
    #[serde(rename = "providerID", default)]
    pub provider_id: String,
    /// Catalog model id.
    #[serde(rename = "modelID", default)]
    pub model_id: String,
    /// Agent to send under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Thinking level to send with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// A scheduled task that queued an item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ScheduledTask {
    /// The project the task belongs to.
    #[serde(rename = "projectId")]
    pub project_id: String,
    /// The task id.
    #[serde(rename = "taskId")]
    pub task_id: String,
}

/// The item a client enqueues.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EnqueueItem {
    /// The model-facing content.
    pub content: String,
    /// The text the strip shows.
    pub text: String,
    /// The model, agent and variant to send with.
    #[serde(rename = "sendConfig")]
    pub send_config: SendConfig,
}

/// Body of `POST /api/message-queue/sessions/:id/items`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnqueueRequest {
    /// The session's directory.
    pub directory: String,
    /// The item to queue.
    pub item: EnqueueItem,
}

/// Answer to an enqueue: the session's new snapshot and the item's id.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EnqueueResult {
    /// The queue revision this answer produced.
    pub revision: u64,
    /// The session's queue after the enqueue.
    pub session: QueueSession,
    /// The new item's id.
    #[serde(rename = "itemId")]
    pub item_id: String,
}

/// A session's queue after a reorder, a remove or a broadcast.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QueueUpdate {
    /// The queue revision this answer produced.
    pub revision: u64,
    /// The session's queue.
    pub session: QueueSession,
}

/// Answer to taking one item: the snapshot plus the taken item, attachments and all.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TakenOne {
    /// The queue revision this answer produced.
    pub revision: u64,
    /// The session's queue without the taken item.
    pub session: QueueSession,
    /// The item that was taken.
    pub item: QueueItem,
}

/// Answer to taking the whole queue.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TakenAll {
    /// The queue revision this answer produced.
    pub revision: u64,
    /// The session's queue, now empty.
    pub session: QueueSession,
    /// The taken items, in queue order.
    pub items: Vec<QueueItem>,
}

/// Body of `PUT /api/message-queue/sessions/:id/order`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReorderRequest {
    /// The complete order of the queue's item ids.
    #[serde(rename = "itemIds")]
    pub item_ids: Vec<String>,
}

#[cfg(test)]
#[path = "queue_test.rs"]
mod queue_test;
