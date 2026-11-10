//! The reducer steps a fixture carries, in the first format's vocabulary.

use oc_client::Event;
use oc_client::types::{Message, Session};
use oc_state::{Action, ConnectionStatus, ServerFailure, SessionId};
use serde::Deserialize;

/// One reducer step of a fixture.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum Step {
    /// A connection change.
    Connection {
        /// Connected or the failure that stopped the server.
        status: ConnectionStep,
    },
    /// A whole session list.
    SessionsLoaded {
        /// The sessions.
        sessions: Vec<Session>,
    },
    /// The user opened a session.
    OpenSession {
        /// Session id.
        session: String,
    },
    /// A session's messages.
    MessagesLoaded {
        /// Session id.
        session: String,
        /// Newest first, as the server sends them.
        messages: Vec<Message>,
    },
    /// Wire events, in order.
    Events {
        /// The events.
        events: Vec<Event>,
    },
}

/// A connection step's payload.
#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub(super) enum ConnectionStep {
    /// The server is connected.
    Connected,
    /// The server failed to start.
    #[serde(rename_all = "camelCase")]
    Failed {
        /// Error text.
        message: String,
        /// Last stderr lines.
        stderr_tail: Vec<String>,
        /// Last log lines.
        log_tail: Vec<String>,
    },
}

impl Step {
    /// Whether this step reports a server that never came up or exited.
    pub(super) fn failed(&self) -> bool {
        matches!(
            self,
            Self::Connection {
                status: ConnectionStep::Failed { .. }
            }
        )
    }
}

/// The reducer actions one step applies, in order.
pub(super) fn actions(step: Step) -> Vec<Action> {
    match step {
        Step::Connection { status } => vec![Action::Connection(connection(status))],
        Step::SessionsLoaded { sessions } => vec![Action::SessionsLoaded(sessions)],
        Step::OpenSession { session } => vec![Action::OpenSession(SessionId::from(session))],
        Step::MessagesLoaded { session, messages } => vec![Action::MessagesLoaded {
            session: SessionId::from(session),
            messages,
        }],
        Step::Events { events } => events.into_iter().map(Action::Event).collect(),
    }
}

fn connection(step: ConnectionStep) -> ConnectionStatus {
    match step {
        ConnectionStep::Connected => ConnectionStatus::Connected,
        ConnectionStep::Failed {
            message,
            stderr_tail,
            log_tail,
        } => ConnectionStatus::Failed(ServerFailure {
            message,
            stderr_tail,
            log_tail,
        }),
    }
}
