//! What the window and the I/O runtime say to each other.

use oc_state::{Action, SessionId};

/// A request the window sends to the server session task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// `GET /api/session`.
    LoadSessions,
    /// `GET /api/session/:id/message`.
    LoadMessages(SessionId),
    /// `POST /api/session`.
    CreateSession,
    /// `POST /api/session/:id/prompt`.
    SendPrompt {
        /// The open session.
        session: SessionId,
        /// The composer text.
        text: String,
    },
}

/// What the server session task reports to the window.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Update {
    /// A change the window applies to its state.
    Apply(Action),
    /// The auth check passed; the window records OpenCode's pids and loads its lists.
    Authenticated,
    /// The event stream is back after a drop; events of the gap are lost, so the window
    /// refetches its lists, and records OpenCode's pids, since the server may have restarted it.
    Reconnected,
}
