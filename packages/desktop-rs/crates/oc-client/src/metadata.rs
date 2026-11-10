//! A session's `metadata`, as `GET /api/session` lists it and `session.metadata.updated`
//! replaces it.
//!
//! Metadata is open-ended: OpenChamber, plugins and other clients write their own keys. The
//! client keeps it as JSON and reads only `openchamber.work.state`, so a key of an unexpected
//! shape never fails a session list.

use serde::Deserialize;
use serde_json::Value;

const WORK_STATE: &str = "/openchamber/work/state";
const PINNED_AT: &str = "/openchamber/pinned/at";
const PINNED_LIST: &str = "/openchamber/pinned/list";
const KIND: &str = "/openchamber/kind";
const ORIGINAL_SESSION: &str = "/openchamber/originalSessionID";
const BTW_KIND: &str = "btw";

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

    /// `openchamber.pinned.at` is a number and a present `list` is a non-empty string;
    /// the pin schema rejects the whole pin otherwise
    /// (`packages/ui/src/lib/sessionPins.ts:26-31`).
    pub fn pinned(&self) -> bool {
        if !self.0.pointer(PINNED_AT).is_some_and(Value::is_number) {
            return false;
        }
        match self.0.pointer(PINNED_LIST) {
            Some(Value::String(list)) => !list.is_empty(),
            Some(_) => false,
            None => true,
        }
    }

    /// `openchamber.pinned.list`, when the pin is filed in one of the user's lists
    /// (`packages/ui/src/lib/sessionPins.ts:33-38`).
    pub fn pin_list(&self) -> Option<&str> {
        self.0.pointer(PINNED_LIST).and_then(Value::as_str)
    }

    /// `openchamber.kind` is `btw` and `originalSessionID` is a non-empty string
    /// (`packages/ui/src/lib/sessionBtwMetadata.ts:55-62`).
    pub fn btw(&self) -> bool {
        let kind = self.0.pointer(KIND).and_then(Value::as_str);
        let original = self.0.pointer(ORIGINAL_SESSION).and_then(Value::as_str);
        kind == Some(BTW_KIND) && original.is_some_and(|id| !id.trim().is_empty())
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
