//! Walks the global session list: one page per request, deduped by id, stopped by the three
//! rules from `packages/ui/src/stores/globalSessions.ts:51-120`.

use std::collections::HashSet;

use crate::types::{Page, Session};

/// What to do after one accepted page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageStep {
    /// Request another page with this cursor.
    Next(String),
    /// The list ended; request nothing more.
    Done,
}

/// The state of one walk over the global session list.
///
/// The walk stops when a page carries no `next` cursor, when `next` repeats the cursor the
/// page was requested with, or when a page adds no id that was not seen before. Retries
/// belong to the caller: one failed request does not end the walk.
#[derive(Debug, Default)]
pub struct SessionPager {
    seen: HashSet<String>,
    sessions: Vec<Session>,
    cursor: Option<String>,
    finished: bool,
}

impl SessionPager {
    /// A walk that has not requested a page yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The cursor to send with the next page; `None` asks for the first page or means the
    /// walk is done.
    pub fn cursor(&self) -> Option<&str> {
        if self.finished {
            None
        } else {
            self.cursor.as_deref()
        }
    }

    /// Accepts one page, keeping every session whose id was not seen before.
    pub fn accept(&mut self, page: Page<Session>) -> PageStep {
        let mut added = 0;
        for session in page.data {
            if session.id.is_empty() || !self.seen.insert(session.id.clone()) {
                continue;
            }
            self.sessions.push(session);
            added += 1;
        }
        let next = page.cursor.next;
        let repeated = next.is_some() && next == self.cursor;
        if added == 0 || repeated {
            self.finished = true;
            return PageStep::Done;
        }
        match next {
            Some(next) => {
                self.cursor = Some(next.clone());
                PageStep::Next(next)
            }
            None => {
                self.finished = true;
                PageStep::Done
            }
        }
    }

    /// The sessions accepted so far, in the order the server listed them.
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// The deduped sessions of the finished walk.
    pub fn into_sessions(self) -> Vec<Session> {
        self.sessions
    }
}

#[cfg(test)]
#[path = "session_page_test.rs"]
mod session_page_test;
