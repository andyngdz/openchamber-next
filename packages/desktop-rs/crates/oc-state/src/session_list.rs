//! The sessions of one view, always in sidebar order.
//!
//! Ordering is [`SessionEntry`]'s: pinned first, then newest activity, newer creation, id
//! (`packages/ui/src/sync/session-ordering.ts:205-228`).

use crate::ids::SessionId;
use crate::session_entry::SessionEntry;
use crate::status::SessionStatus;

/// The sessions of one view, always in sidebar order.
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

    /// The view has no session.
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
    /// session that stays listed keeps them while a turn runs; a session that is idle after
    /// the merge drops them, since nothing waits for an answer outside a turn, and a reply
    /// event missed while disconnected would otherwise keep it waiting for good. Activity
    /// never moves back. The listed
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
                    if entry.status != SessionStatus::Idle {
                        entry.pending.clone_from(&old.pending);
                    }
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
#[path = "session_list_test.rs"]
mod session_list_test;
