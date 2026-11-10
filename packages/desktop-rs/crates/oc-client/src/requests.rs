//! Request and record bodies of the session routes.

use serde::{Deserialize, Serialize};

use crate::types::{Location, ModelRef};

/// Body of `POST /api/session`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateSessionRequest {
    /// Directory the new session is created in.
    pub location: Location,
    /// Agent the session starts on; absent when the user picked none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Model the session starts on; absent when the user picked none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
}

/// Body of `POST /api/session/:id/model`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SwitchModelRequest {
    /// The model to switch the session to.
    pub model: ModelRef,
}

/// Body of `POST /api/session/:id/agent`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SwitchAgentRequest {
    /// The agent to switch the session to.
    pub agent: String,
}

/// `GET /api/session/:id`: the record's agent and model, which the list does not carry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionRecord {
    /// `ses_…` id.
    pub id: String,
    /// The agent the session is on.
    #[serde(default)]
    pub agent: Option<String>,
    /// The model the session is on; `variant` is the Thinking level.
    #[serde(default)]
    pub model: Option<ModelRef>,
}

#[cfg(test)]
#[path = "requests_test.rs"]
mod requests_test;
