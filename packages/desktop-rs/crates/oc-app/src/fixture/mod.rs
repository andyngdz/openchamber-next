//! `fixtures/states/<name>.json`: a window's directory and the actions that build its state.
//!
//! Each step is one [`Action`] for the reducer, or a run of wire events. Sessions, messages
//! and events keep the server's JSON shape, so they decode through `oc-client`'s types the
//! same way live answers do.

use std::path::Path;

use oc_client::Event;
use oc_client::types::{Message, Session};
use oc_state::{Action, ConnectionStatus, ServerFailure, SessionId};
use serde::Deserialize;

use crate::errors::AppError;

/// A decoded state fixture.
#[derive(Debug)]
pub(crate) struct StateScript {
    /// The project directory the window shows.
    pub(crate) directory: String,
    /// Actions in the order the reducer applies them.
    pub(crate) actions: Vec<Action>,
}

#[derive(Deserialize)]
struct StateFile {
    directory: String,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Step {
    Connection {
        status: ConnectionStep,
    },
    SessionsLoaded {
        sessions: Vec<Session>,
    },
    OpenSession {
        session: String,
    },
    MessagesLoaded {
        session: String,
        messages: Vec<Message>,
    },
    Events {
        events: Vec<Event>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum ConnectionStep {
    Connected,
    #[serde(rename_all = "camelCase")]
    Failed {
        message: String,
        stderr_tail: Vec<String>,
        log_tail: Vec<String>,
    },
}

impl StateScript {
    /// Reads and decodes the fixture file at `path`, as given on the command line.
    pub(crate) fn load(path: &Path) -> Result<Self, AppError> {
        let text = std::fs::read_to_string(path).map_err(|source| AppError::ReadFixture {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text).map_err(|source| AppError::DecodeFixture {
            path: path.to_path_buf(),
            source,
        })
    }

    fn parse(text: &str) -> Result<Self, serde_json::Error> {
        let file: StateFile = serde_json::from_str(text)?;
        let actions = file.steps.into_iter().flat_map(step_actions).collect();
        Ok(Self {
            directory: file.directory,
            actions,
        })
    }
}

fn step_actions(step: Step) -> Vec<Action> {
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

#[cfg(test)]
#[path = "fixture_test.rs"]
pub(crate) mod fixture_test;
