//! The directory's sessions in the order the sidebar shows them.
//!
//! The React sidebar orders a directory's sessions newest activity first
//! (`compareSessionsByLifecycleOrder` in `packages/ui/src/sync/session-ordering.ts`): a session
//! moves to the top when it starts or ends a turn, and otherwise sits at its last activity time,
//! ties broken by newer creation, then by id. Here a turn start or end stamps the event's
//! `created`, and a loaded session starts at `time.idle`, else `time.updated`, else
//! `time.created`, as `getSessionActivityTime` does; a rename does not move a session in either.
//! Pinned sessions, which React lists first, do not exist in the spike.

use std::cmp::Reverse;

use oc_client::types::WorkState;

use crate::ids::{RequestId, SessionId};
use crate::status::SessionStatus;

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
    activity: u64,
    created: u64,
    /// When `status` was last set (ms): an event's time, or the newest turn stamp of a list.
    status_at: u64,
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
            activity,
            created,
            status_at: 0,
        }
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
    /// `isSessionInWork` in `packages/ui/src/lib/sessionWorkMetadata.ts` decides.
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

    fn order_key(&self) -> (Reverse<u64>, Reverse<u64>, &SessionId) {
        (Reverse(self.activity), Reverse(self.created), &self.id)
    }
}

/// The sessions of the directory, always in sidebar order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionList {
    entries: Vec<SessionEntry>,
}

impl SessionList {
    /// The rows in sidebar order.
    pub fn iter(&self) -> impl Iterator<Item = &SessionEntry> {
        self.entries.iter()
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The directory has no session.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The row of this session.
    pub fn get(&self, id: &SessionId) -> Option<&SessionEntry> {
        self.entries.iter().find(|entry| entry.id == *id)
    }

    pub(crate) fn contains(&self, id: &str) -> bool {
        self.entries.iter().any(|entry| entry.id == *id)
    }

    pub(crate) fn get_mut(&mut self, id: &str) -> Option<&mut SessionEntry> {
        self.entries.iter_mut().find(|entry| entry.id == *id)
    }

    /// Adds a row unless the session is listed already.
    pub(crate) fn insert(&mut self, entry: SessionEntry) {
        if self.contains(entry.id.as_str()) {
            return;
        }
        self.entries.push(entry);
        self.sort();
    }

    pub(crate) fn remove(&mut self, id: &str) {
        self.entries.retain(|entry| entry.id != *id);
    }

    /// Replaces the rows with a fresh list. Pending requests come only from events, so a
    /// session that stays listed keeps them, and its activity never moves back. The listed
    /// status wins only when its turn stamp is newer than the status already held, so a
    /// reload corrects a status whose ending event was missed, while an older snapshot never
    /// overrides a newer event.
    pub(crate) fn replace(&mut self, fresh: Vec<SessionEntry>) {
        let previous = std::mem::take(&mut self.entries);
        self.entries = fresh
            .into_iter()
            .map(|mut entry| {
                if let Some(old) = previous.iter().find(|old| old.id == entry.id) {
                    if entry.status_at <= old.status_at {
                        entry.status = old.status;
                        entry.status_at = old.status_at;
                    }
                    entry.pending.clone_from(&old.pending);
                    entry.activity = entry.activity.max(old.activity);
                }
                entry
            })
            .collect();
        self.sort();
    }

    /// Moves the session to its new activity time, as a turn start or end does.
    pub(crate) fn touch(&mut self, id: &str, at: u64) {
        if let Some(entry) = self.get_mut(id) {
            entry.activity = entry.activity.max(at);
            self.sort();
        }
    }

    fn sort(&mut self) {
        self.entries
            .sort_by(|left, right| left.order_key().cmp(&right.order_key()));
    }
}

#[cfg(test)]
#[path = "work_test.rs"]
mod work_test;
