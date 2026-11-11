//! Queue, auto-accept and routing calls: the server-owned message queue, the permission
//! policy and the routing state the safety net depends on.

use crate::client::Client;
use crate::directory::Directory;
use crate::error::ClientError;
use crate::queue::{
    EnqueueItem, EnqueueRequest, EnqueueResult, MessageQueue, QueueUpdate, ReorderRequest,
    TakenAll, TakenOne,
};
use crate::routing::{AutoAcceptRequest, AutoAcceptSnapshot, PermissionMode, RoutingState};
use crate::uri_component::encode_uri_component;

const QUEUE: &str = "/api/message-queue";
const AUTO_ACCEPT: &str = "/api/permission-auto-accept";
const ROUTING: &str = "/api/routing";

impl Client {
    /// `GET /api/message-queue`: every session's queue at the server's revision.
    pub async fn message_queue(&self) -> Result<MessageQueue, ClientError> {
        self.get_unscoped(QUEUE).await
    }

    /// `POST /api/message-queue/sessions/:id/items` with the session's directory, the item
    /// and the composer's send config.
    pub async fn enqueue(
        &self,
        directory: &Directory,
        session_id: &str,
        item: &EnqueueItem,
    ) -> Result<EnqueueResult, ClientError> {
        let body = EnqueueRequest {
            directory: directory.as_str().to_owned(),
            item: item.clone(),
        };
        self.send(
            self.post_scoped(&queue_path(session_id, "/items"), directory)
                .json(&body),
        )
        .await
    }

    /// `POST /api/message-queue/sessions/:id/take`: every queued item, in queue order.
    pub async fn take_all(&self, session_id: &str) -> Result<TakenAll, ClientError> {
        self.send(self.post_unscoped(&queue_path(session_id, "/take")))
            .await
    }

    /// `POST /api/message-queue/sessions/:id/items/:itemId/take`: one queued item.
    pub async fn take_item(
        &self,
        session_id: &str,
        item_id: &str,
    ) -> Result<TakenOne, ClientError> {
        let suffix = format!("/items/{}/take", encode_uri_component(item_id));
        self.send(self.post_unscoped(&queue_path(session_id, &suffix)))
            .await
    }

    /// `PUT /api/message-queue/sessions/:id/order` with the complete new order.
    pub async fn reorder(
        &self,
        session_id: &str,
        item_ids: &[String],
    ) -> Result<QueueUpdate, ClientError> {
        let body = ReorderRequest {
            item_ids: item_ids.to_vec(),
        };
        self.send(
            self.put_unscoped(&queue_path(session_id, "/order"))
                .json(&body),
        )
        .await
    }

    /// `DELETE /api/message-queue/sessions/:id/items/:itemId`. An item the server is
    /// sending answers 409, which stays a [`ClientError::Status`].
    pub async fn remove_item(
        &self,
        session_id: &str,
        item_id: &str,
    ) -> Result<QueueUpdate, ClientError> {
        let suffix = format!("/items/{}", encode_uri_component(item_id));
        self.send(self.delete_unscoped(&queue_path(session_id, &suffix)))
            .await
    }

    /// `GET /api/permission-auto-accept`: every session's mode and the policy revision.
    pub async fn auto_accept(&self) -> Result<AutoAcceptSnapshot, ClientError> {
        self.get_unscoped(AUTO_ACCEPT).await
    }

    /// `PUT /api/permission-auto-accept/sessions/:id`: set one session's mode; the answer
    /// is the whole policy.
    pub async fn set_auto_accept(
        &self,
        directory: &Directory,
        session_id: &str,
        mode: PermissionMode,
    ) -> Result<AutoAcceptSnapshot, ClientError> {
        let body = AutoAcceptRequest {
            mode,
            enabled: mode != PermissionMode::Ask,
            directory: directory.as_str().to_owned(),
        };
        let path = format!(
            "{AUTO_ACCEPT}/sessions/{}",
            encode_uri_component(session_id)
        );
        self.send(self.put_unscoped(&path).json(&body)).await
    }

    /// `GET /api/routing`: what the safety decisions read. A 404 means the build has no
    /// routing and reads as [`RoutingState::unavailable`].
    pub async fn routing(&self) -> Result<RoutingState, ClientError> {
        match self.get_unscoped(ROUTING).await {
            Ok(state) => Ok(state),
            Err(ClientError::NotFound { .. }) => Ok(RoutingState::unavailable()),
            Err(error) => Err(error),
        }
    }
}

/// `/api/message-queue/sessions/:id` with a suffix such as `/order`.
fn queue_path(session_id: &str, suffix: &str) -> String {
    format!(
        "{QUEUE}/sessions/{}{suffix}",
        encode_uri_component(session_id)
    )
}

#[cfg(test)]
#[path = "queue_calls_test.rs"]
mod queue_calls_test;
