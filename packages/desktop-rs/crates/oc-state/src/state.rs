//! Everything the window draws, in one value the app replaces only through [`State::apply`].

use crate::ids::SessionId;
use crate::message::ChatMessage;
use crate::session_list::{SessionEntry, SessionList};
use crate::status::{ComposerStatus, ConnectionStatus, LoadStatus, Region, SessionStatus};

/// The open session and its messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSession {
    /// Session id.
    pub id: SessionId,
    /// User and assistant messages, oldest first.
    pub messages: Vec<ChatMessage>,
    /// Load state of the messages; the chat column shows its error.
    pub load: LoadStatus,
}

/// The composer's text and what is happening to it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Composer {
    /// What the user typed; kept until the server accepts the prompt.
    pub draft: String,
    /// Whether a prompt is in flight or failed.
    pub status: ComposerStatus,
}

/// View state of one window: one directory, its sessions, the open session, the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub(crate) directory: String,
    pub(crate) connection: ConnectionStatus,
    pub(crate) sessions: SessionList,
    pub(crate) sidebar: LoadStatus,
    pub(crate) open: Option<OpenSession>,
    pub(crate) composer: Composer,
    /// Why the last `POST /api/session` failed, until a create succeeds or is retried.
    pub(crate) create_error: Option<String>,
}

impl State {
    /// Empty state for `directory`, server starting, session list not loaded.
    pub fn new(directory: impl Into<String>) -> Self {
        Self {
            directory: directory.into(),
            connection: ConnectionStatus::Starting,
            sessions: SessionList::default(),
            sidebar: LoadStatus::Loading,
            open: None,
            composer: Composer::default(),
            create_error: None,
        }
    }

    /// The project directory the window shows.
    pub fn directory(&self) -> &str {
        &self.directory
    }

    /// Server state for the header.
    pub fn connection(&self) -> &ConnectionStatus {
        &self.connection
    }

    /// The directory's sessions in sidebar order.
    pub fn sessions(&self) -> &SessionList {
        &self.sessions
    }

    /// Load state of the session list.
    pub fn sidebar(&self) -> &LoadStatus {
        &self.sidebar
    }

    /// The open session, `None` on the empty state.
    pub fn open(&self) -> Option<&OpenSession> {
        self.open.as_ref()
    }

    /// The composer.
    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    /// The sidebar row of the open session.
    pub fn open_entry(&self) -> Option<&SessionEntry> {
        self.open
            .as_ref()
            .and_then(|open| self.sessions.get(&open.id))
    }

    /// Status of the open session; a session the list does not hold counts as idle.
    pub fn open_status(&self) -> SessionStatus {
        self.open_entry()
            .map_or(SessionStatus::Idle, |entry| entry.status)
    }

    /// The agent waits for an answer in the open session.
    pub fn open_is_waiting(&self) -> bool {
        self.open_entry().is_some_and(SessionEntry::is_waiting)
    }

    /// Error text a region shows, if its last request failed.
    pub fn region_error(&self, region: Region) -> Option<&str> {
        match region {
            Region::Sidebar => load_error(&self.sidebar),
            Region::Messages => self.open.as_ref().and_then(|open| load_error(&open.load)),
            Region::Composer => match &self.composer.status {
                ComposerStatus::Failed { message } => Some(message),
                ComposerStatus::Idle | ComposerStatus::Sending => None,
            },
            Region::NewSession => self.create_error.as_deref(),
        }
    }

    /// The send button is enabled: connected, a session open and not busy, nothing in flight,
    /// and text typed.
    pub fn can_send(&self) -> bool {
        self.connection == ConnectionStatus::Connected
            && self.open.is_some()
            && self.open_status() != SessionStatus::Busy
            && self.composer.status != ComposerStatus::Sending
            && !self.composer.draft.trim().is_empty()
    }

    /// The session is listed or open; events of any other session are dropped.
    pub(crate) fn is_known(&self, session_id: &str) -> bool {
        self.sessions.contains(session_id) || self.is_open(session_id)
    }

    pub(crate) fn is_open(&self, session_id: &str) -> bool {
        self.open
            .as_ref()
            .is_some_and(|open| open.id == *session_id)
    }

    /// Messages of the open session when it is `session_id`.
    pub(crate) fn open_messages_mut(&mut self, session_id: &str) -> Option<&mut Vec<ChatMessage>> {
        self.open
            .as_mut()
            .filter(|open| open.id == *session_id)
            .map(|open| &mut open.messages)
    }
}

fn load_error(status: &LoadStatus) -> Option<&str> {
    match status {
        LoadStatus::Failed { message } => Some(message),
        LoadStatus::Loading | LoadStatus::Ready => None,
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod state_test;
