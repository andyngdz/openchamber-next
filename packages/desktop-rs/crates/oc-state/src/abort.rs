//! The Escape-twice abort window, per session (`hooks/useKeyboardShortcuts.ts:452-499`;
//! design "Abort prompt").
//!
//! The first Escape arms the session for [`ABORT_WINDOW_MS`]; a second Escape for the same
//! session inside that window confirms and stops the turn. Equality is not inside the
//! window: React requires `now < primedUntil`. The app decides when an Escape counts at all
//! (composer focus, no popup, no IME composition, session busy); the window only remembers
//! the first one.

use crate::ids::SessionId;
use crate::state::State;

/// How long the second Escape has to follow the first.
pub const ABORT_WINDOW_MS: u64 = 3000;

/// The armed session and when its window closes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AbortWindow {
    armed: Option<(String, u64)>,
}

impl AbortWindow {
    /// Starts or restarts the window for `session` at `now`.
    fn arm(&mut self, session: &str, now: u64) {
        self.armed = Some((session.to_owned(), now + ABORT_WINDOW_MS));
    }

    /// Confirms a second Escape for the same session inside the window, and clears it.
    fn confirm(&mut self, session: &str, now: u64) -> bool {
        let Some((armed, expires_at)) = self.armed.as_ref() else {
            return false;
        };
        if armed != session || now >= *expires_at {
            return false;
        }
        self.armed = None;
        true
    }
}

impl State {
    /// Arms the abort window for `session`; the app calls this on the first Escape.
    pub fn arm_abort(&mut self, session: &SessionId, now: u64) {
        self.abort.arm(session.as_str(), now);
    }

    /// Whether this Escape is the second one inside the window; a confirmation clears it.
    pub fn confirm_abort(&mut self, session: &SessionId, now: u64) -> bool {
        self.abort.confirm(session.as_str(), now)
    }
}

#[cfg(test)]
#[path = "abort_test.rs"]
mod abort_test;
