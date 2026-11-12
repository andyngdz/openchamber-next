//! The server-owned message queue, by session (`/api/message-queue`; design "Queue").
//!
//! Every snapshot carries the server revision it was applied at. A session's snapshot is
//! skipped only when its revision is older than the one held for that session, so an
//! action's answer and its broadcast at the same revision both apply
//! (`packages/ui/src/stores/messageQueueStore.ts:524-552`). A full `GET` replaces every
//! session at its revision, and a session a broadcast already moved past that revision
//! stays as the broadcast left it.

use std::collections::{BTreeMap, BTreeSet};

use oc_client::queue::{MessageQueue, QueueItem, QueueSession, QueueUpdate};

use crate::ids::SessionId;
use crate::state::State;

/// One session's queue and the revision that queue was applied at.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HeldQueue {
    /// The server revision this session's queue reflects.
    pub(crate) revision: u64,
    /// The queue itself.
    pub(crate) session: QueueSession,
}

/// Every session's queue, each at its own revision.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Queues {
    sessions: BTreeMap<String, HeldQueue>,
}

impl Queues {
    /// Applies one session's snapshot; returns false when it is older than the held one.
    ///
    /// Equality applies: an action's answer and its broadcast share the revision.
    pub(crate) fn apply(&mut self, revision: u64, session: QueueSession) -> bool {
        let id = session.session_id.clone();
        if let Some(held) = self.sessions.get(&id)
            && held.revision > revision
        {
            return false;
        }
        self.sessions.insert(id, HeldQueue { revision, session });
        true
    }

    /// Replaces every session with a full `GET`'s state at its revision.
    ///
    /// A session the snapshot does not list is cleared, unless a broadcast already moved
    /// it past the snapshot's revision.
    pub(crate) fn apply_full(&mut self, snapshot: MessageQueue) {
        let revision = snapshot.revision;
        let mut listed: BTreeSet<String> = BTreeSet::new();
        for session in snapshot.sessions {
            listed.insert(session.session_id.clone());
            self.apply(revision, session);
        }
        self.sessions
            .retain(|id, held| listed.contains(id) || held.revision > revision);
    }

    /// One session's held queue.
    pub(crate) fn get(&self, session: &str) -> Option<&HeldQueue> {
        self.sessions.get(session)
    }
}

impl State {
    /// `GET /api/message-queue` answered: every session at the server's revision.
    pub fn queue_loaded(&mut self, snapshot: MessageQueue) {
        self.queues.apply_full(snapshot);
    }

    /// `openchamber:message-queue.updated`, or an action's own answer: one session's queue.
    pub fn queue_updated(&mut self, update: &QueueUpdate) {
        self.queues.apply(update.revision, update.session.clone());
    }

    /// The session's held queue, when any snapshot named it.
    pub fn queue(&self, session: &SessionId) -> Option<&QueueSession> {
        self.queues.get(session.as_str()).map(|held| &held.session)
    }

    /// The session's queued items in queue order; empty when it holds none.
    pub fn queue_items(&self, session: &SessionId) -> &[QueueItem] {
        self.queues
            .get(session.as_str())
            .map_or(&[], |held| held.session.items.as_slice())
    }

    /// The revision the session's queue reflects, once a snapshot named it.
    pub fn queue_revision(&self, session: &SessionId) -> Option<u64> {
        self.queues.get(session.as_str()).map(|held| held.revision)
    }

    /// Whether the server is sending this item right now; such a row offers no edit or remove.
    pub fn queue_is_sending(&self, session: &SessionId, item_id: &str) -> bool {
        self.queues
            .get(session.as_str())
            .and_then(|held| held.session.sending_id.as_deref())
            == Some(item_id)
    }
}

#[cfg(test)]
#[path = "queue_test.rs"]
mod queue_test;
