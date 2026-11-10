//! A session's `metadata`, as `GET /api/session` lists it and `session.metadata.updated`
//! replaces it.
//!
//! Metadata is open-ended: OpenChamber, plugins and other clients write their own keys. The
//! client keeps it as JSON and reads only `openchamber.work.state`, so a key of an unexpected
//! shape never fails a session list.

use serde::Deserialize;
use serde_json::Value;

const WORK_STATE: &str = "/openchamber/work/state";

/// Whether a session is tracked as work, `metadata.openchamber.work.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    /// The session is in work: the sidebar lists it under "In work".
    Open,
    /// The user marked the work done.
    Done,
}

/// The session's metadata object, kept whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SessionMetadata(Value);

impl SessionMetadata {
    /// The work state, `None` when the metadata has none or a value this client does not know.
    ///
    /// # Examples
    ///
    /// ```
    /// use oc_client::types::{SessionMetadata, WorkState};
    /// let metadata: SessionMetadata =
    ///     serde_json::from_str(r#"{"openchamber":{"work":{"state":"open"}}}"#)?;
    /// assert_eq!(metadata.work_state(), Some(WorkState::Open));
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn work_state(&self) -> Option<WorkState> {
        match self.0.pointer(WORK_STATE).and_then(Value::as_str)? {
            "open" => Some(WorkState::Open),
            "done" => Some(WorkState::Done),
            _ => None,
        }
    }
}

/// `session.metadata.updated`: the session's whole metadata, which replaces the old one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MetadataUpdated {
    /// The session whose metadata changed.
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// The new metadata.
    #[serde(default)]
    pub metadata: SessionMetadata,
}

#[cfg(test)]
#[path = "metadata_test.rs"]
mod metadata_test;
