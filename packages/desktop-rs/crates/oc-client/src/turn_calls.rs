//! Turn-control calls: a directory's pending permissions and forms, their replies and
//! cancels, Stop, and the steer prompt.

use crate::calls::{PROMPT, session_path};
use crate::client::Client;
use crate::directory::Directory;
use crate::error::ClientError;
use crate::form::{FormAnswer, FormInfo, FormReplyRequest};
use crate::permission::{PermissionReply, PermissionReplyRequest, PermissionRequest};
use crate::prompt::{Delivery, PromptAccepted, PromptRequest};
use crate::types::{Data, InterruptResult, Located};
use crate::uri_component::encode_uri_component;

const PERMISSIONS: &str = "/api/permission/request";
const FORMS: &str = "/api/form";

impl Client {
    /// `GET /api/permission/request?location[directory]=`: the directory's pending
    /// requests, `global` ones excluded because only forms carry them.
    pub async fn permissions(
        &self,
        directory: &Directory,
    ) -> Result<Vec<PermissionRequest>, ClientError> {
        let path = located_path(PERMISSIONS, directory);
        let located: Located<Vec<PermissionRequest>> = self.get_scoped(&path, directory).await?;
        Ok(located.data)
    }

    /// `POST /api/session/:id/permission/:requestId/reply` with one decision. A 404 means
    /// the request is already settled ([`ClientError::NotFound`]).
    pub async fn reply_permission(
        &self,
        directory: &Directory,
        session_id: &str,
        request_id: &str,
        decision: PermissionReply,
    ) -> Result<(), ClientError> {
        let body = PermissionReplyRequest { decision };
        let suffix = format!("/permission/{}/reply", encode_uri_component(request_id));
        let path = session_path(session_id, &suffix);
        self.send_empty(self.post_scoped(&path, directory).json(&body))
            .await
    }

    /// `GET /api/form?location[directory]=`: the directory's pending forms, including the
    /// `global` ones.
    pub async fn forms(&self, directory: &Directory) -> Result<Vec<FormInfo>, ClientError> {
        let path = located_path(FORMS, directory);
        let located: Located<Vec<FormInfo>> = self.get_scoped(&path, directory).await?;
        Ok(located.data)
    }

    /// `POST /api/session/:id/form/:formId/reply` with the active fields' answers. A 404
    /// means the form is already settled ([`ClientError::NotFound`]).
    pub async fn reply_form(
        &self,
        directory: &Directory,
        session_id: &str,
        form_id: &str,
        answer: &FormAnswer,
    ) -> Result<(), ClientError> {
        let body = FormReplyRequest {
            answer: answer.clone(),
        };
        let suffix = format!("/form/{}/reply", encode_uri_component(form_id));
        let path = session_path(session_id, &suffix);
        self.send_empty(self.post_scoped(&path, directory).json(&body))
            .await
    }

    /// `DELETE /api/session/:id/form/:formId`. A 404 means the form is already settled
    /// ([`ClientError::NotFound`]).
    pub async fn cancel_form(
        &self,
        directory: &Directory,
        session_id: &str,
        form_id: &str,
    ) -> Result<(), ClientError> {
        let suffix = format!("/form/{}", encode_uri_component(form_id));
        let path = session_path(session_id, &suffix);
        self.send_empty(self.delete_scoped(&path, directory)).await
    }

    /// `POST /api/session/:id/interrupt`: stop a running turn. `interrupted` is `false`
    /// when nothing was running.
    pub async fn interrupt(
        &self,
        directory: &Directory,
        session_id: &str,
    ) -> Result<InterruptResult, ClientError> {
        self.send(self.post_scoped(&session_path(session_id, "/interrupt"), directory))
            .await
    }

    /// `POST /api/session/:id/prompt` with `delivery: "steer"`, the send into a running
    /// turn. [`Client::send_prompt`] keeps the plain send's signature.
    pub async fn steer_prompt(
        &self,
        directory: &Directory,
        session_id: &str,
        text: &str,
    ) -> Result<PromptAccepted, ClientError> {
        let body = PromptRequest {
            id: crate::new_message_id(),
            text: text.to_owned(),
            delivery: Some(Delivery::Steer),
        };
        let accepted: Data<PromptAccepted> = self
            .send(
                self.post_scoped(&session_path(session_id, PROMPT), directory)
                    .json(&body),
            )
            .await?;
        Ok(accepted.data)
    }
}

/// A list path carrying the directory in the `location[directory]` query the list routes
/// read, alongside the directory header.
fn located_path(path: &str, directory: &Directory) -> String {
    format!(
        "{path}?location[directory]={}",
        encode_uri_component(directory.as_str())
    )
}

#[cfg(test)]
#[path = "turn_calls_test.rs"]
mod turn_calls_test;
