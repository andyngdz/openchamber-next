//! Which sessions the sidebar lists under "In work".
//!
//! The React sidebar reads `metadata.openchamber.work.state` of each session: `open` puts it
//! in the In work group, anything else back under its project. `oc-state` does not model this
//! yet, so the app follows `session.metadata.updated` events, which `oc-state` drops as
//! unknown, before it hands each action to the reducer.

use std::collections::HashSet;

use oc_client::EventData;
use oc_state::{Action, SessionId};
use serde::Deserialize;

const METADATA_UPDATED: &str = "session.metadata.updated";

/// Sessions whose work state is open.
#[derive(Debug, Default)]
pub(crate) struct WorkTracker {
    open: HashSet<SessionId>,
}

#[derive(Deserialize)]
struct MetadataUpdated {
    #[serde(rename = "sessionID")]
    session_id: String,
    #[serde(default)]
    metadata: Metadata,
}

#[derive(Deserialize, Default)]
struct Metadata {
    #[serde(default)]
    openchamber: Option<OpenChamberMetadata>,
}

#[derive(Deserialize)]
struct OpenChamberMetadata {
    #[serde(default)]
    work: Option<Work>,
}

#[derive(Deserialize)]
struct Work {
    state: WorkState,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum WorkState {
    Open,
    #[serde(other)]
    Closed,
}

impl WorkTracker {
    /// Follows the work state carried by a metadata event; other actions change nothing.
    pub(crate) fn observe(&mut self, action: &Action) {
        let Action::Event(event) = action else {
            return;
        };
        let EventData::Raw { kind, data } = &event.data else {
            return;
        };
        if kind != METADATA_UPDATED {
            return;
        }
        let Ok(update) = MetadataUpdated::deserialize(data) else {
            tracing::debug!(
                target: crate::logging::APP,
                "[APP] Dropped a session.metadata.updated event that does not decode"
            );
            return;
        };
        let Some(work) = update.metadata.openchamber.and_then(|meta| meta.work) else {
            return;
        };
        let id = SessionId::from(update.session_id);
        match work.state {
            WorkState::Open => self.open.insert(id),
            WorkState::Closed => self.open.remove(&id),
        };
    }

    /// The session sits in the In work group.
    pub(crate) fn is_in_work(&self, id: &SessionId) -> bool {
        self.open.contains(id)
    }
}

#[cfg(test)]
#[path = "work_test.rs"]
mod work_test;
