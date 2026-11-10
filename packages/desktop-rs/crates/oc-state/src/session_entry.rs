//! One sidebar row: the session facts the list orders and the sidebar splits on.
//!
//! The React sidebar orders sessions newest activity first
//! (`compareSessionsByLifecycleOrder` in `packages/ui/src/sync/session-ordering.ts:205-228`): a
//! pinned session first, then a session by its last activity time, ties broken by newer
//! creation, then by id. The sidebar passes the empty rank table, so the lifecycle rank is the
//! activity time alone (`sessionSidebarRowModel.ts:352`). Here a turn start or end stamps the
//! event's `created`, and a loaded session starts at `time.idle`, else `time.updated`, else
//! `time.created`, as `getSessionActivityTime` does; a rename does not move a session.

use std::cmp::Reverse;

use oc_client::types::WorkState;

use crate::ids::{RequestId, SessionId};
use crate::status::SessionStatus;

/// The name React gives the default pin list (`packages/ui/src/lib/sidebarSections.ts:32`).
const DEFAULT_PIN_LIST: &str = "pinned";

/// A request from the agent the user has not answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingRequest {
    /// A `permission.asked` request.
    Permission(RequestId),
    /// A `form.created` form.
    Form(RequestId),
}

/// One sidebar row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    /// Session id.
    pub id: SessionId,
    /// `None` until the first turn has been titled.
    pub title: Option<String>,
    /// Whether a turn runs; drives the busy indicator.
    pub status: SessionStatus,
    /// Unanswered permission requests and forms, oldest first.
    pub pending: Vec<PendingRequest>,
    /// `metadata.openchamber.work.state`, from the list or the latest metadata event.
    pub work: Option<WorkState>,
    /// Absolute directory the session lives in.
    pub directory: String,
    /// `time.archived` was stamped: the session shows nowhere (`globalSessions.ts:26`).
    pub archived: bool,
    /// `metadata.openchamber.pinned.at` is set (`sessionPins.ts:29-31`).
    pub pinned: bool,
    /// `metadata.openchamber.pinned.list`, when the pin is filed in a list.
    pub pin_list: Option<String>,
    /// A `/btw` fork, which never leaks into a sidebar projection (`sessionBtwMetadata.ts:55-57`).
    pub btw: bool,
    /// The session this one nests under, if any.
    pub parent_id: Option<String>,
    pub(crate) activity: u64,
    pub(crate) created: u64,
    pub(crate) updated: u64,
    /// When `status` was last set (ms): an event's time, or the newest turn stamp of a list.
    pub(crate) status_at: u64,
}

impl SessionEntry {
    /// A row created at `created` (ms) and last active at `activity` (ms).
    pub(crate) fn new(id: SessionId, title: Option<String>, created: u64, activity: u64) -> Self {
        Self {
            id,
            title,
            status: SessionStatus::Idle,
            pending: Vec::new(),
            work: None,
            directory: String::new(),
            archived: false,
            pinned: false,
            pin_list: None,
            btw: false,
            parent_id: None,
            activity,
            created,
            updated: activity,
            status_at: 0,
        }
    }

    /// Sets the directory the session lives in.
    pub(crate) fn with_directory(mut self, directory: impl Into<String>) -> Self {
        self.directory = directory.into();
        self
    }

    /// Sets `time.updated`, which the "recent" project order reads (`projectSort.ts:57-74`).
    pub(crate) fn with_updated(mut self, updated: u64) -> Self {
        self.updated = updated;
        self
    }

    /// Sets the facts the shared session list carries besides its times.
    pub(crate) fn with_split_facts(
        mut self,
        archived: bool,
        pinned: bool,
        pin_list: Option<String>,
        btw: bool,
        parent_id: Option<String>,
    ) -> Self {
        self.archived = archived;
        self.pinned = pinned;
        self.pin_list = pin_list;
        self.btw = btw;
        self.parent_id = parent_id;
        self
    }

    /// Sets the status the list reports, stamped with the turn time it was read from.
    pub(crate) fn with_listed_status(mut self, status: SessionStatus, at: u64) -> Self {
        self.status = status;
        self.status_at = at;
        self
    }

    /// Sets the status an event reports at `at` (ms).
    pub(crate) fn record_status(&mut self, status: SessionStatus, at: u64) {
        self.status = status;
        self.status_at = self.status_at.max(at);
    }

    /// Sets the work state the listed metadata reports.
    pub(crate) fn with_work(mut self, work: Option<WorkState>) -> Self {
        self.work = work;
        self
    }

    /// The sidebar lists the session under "In work": its work state is open, as
    /// `isSessionInWork` in `packages/ui/src/lib/sessionWorkMetadata.ts:31-33` decides.
    pub fn is_in_work(&self) -> bool {
        self.work == Some(WorkState::Open)
    }

    /// The busy indicator.
    pub fn is_busy(&self) -> bool {
        self.status == SessionStatus::Busy
    }

    /// The agent is waiting for an answer.
    pub fn is_waiting(&self) -> bool {
        !self.pending.is_empty()
    }

    /// The session is filed in the Pinned list, not one of the user's own pin lists.
    /// `getSessionPinList` returns the default list for a pin without one
    /// (`packages/ui/src/lib/sessionPins.ts:33-38`).
    pub fn is_pinned_in_default_list(&self) -> bool {
        self.pinned
            && self
                .pin_list
                .as_deref()
                .is_none_or(|list| list == DEFAULT_PIN_LIST)
    }

    /// `time.updated`, for the "recent" project order.
    pub fn updated(&self) -> u64 {
        self.updated
    }

    pub(crate) fn order_key(&self) -> (bool, Reverse<u64>, Reverse<u64>, &SessionId) {
        (
            !self.pinned,
            Reverse(self.activity),
            Reverse(self.created),
            &self.id,
        )
    }
}

#[cfg(test)]
#[path = "work_test.rs"]
mod work_test;
