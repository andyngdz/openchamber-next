//! Everything that changes [`State`]: wire events and what the app learns from its own calls.

use oc_client::Event;
use oc_client::types::{Message, Session};

use crate::events::reduce_event;
use crate::ids::SessionId;
use crate::load::{chat_messages, session_entry};
use crate::state::{OpenSession, State};
use crate::status::{ComposerStatus, ConnectionStatus, LoadStatus, Region};

/// One change to the state.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A frame of `GET /api/event`.
    Event(Event),
    /// The server or the event stream changed state.
    Connection(ConnectionStatus),
    /// `GET /api/session` answered; replaces the list.
    SessionsLoaded(Vec<Session>),
    /// `GET /api/session` failed; the rows on screen stay under the error.
    SessionsFailed(String),
    /// `POST /api/session` answered: the session joins the list and opens, with no messages.
    SessionCreated(Session),
    /// `POST /api/session` failed; the sidebar shows the error with a Retry that creates again.
    CreateFailed(String),
    /// The user picked a session; its messages load next.
    OpenSession(SessionId),
    /// `GET /api/session/:id/message` answered for `session`.
    MessagesLoaded {
        /// The session the request was for; a stale answer for another one is dropped.
        session: SessionId,
        /// The answer, newest first as the server sends it.
        messages: Vec<Message>,
    },
    /// `GET /api/session/:id/message` failed for `session`.
    MessagesFailed {
        /// The session the request was for.
        session: SessionId,
        /// Error text for the chat column.
        message: String,
    },
    /// The user pressed retry in a region; the app sends that region's request again.
    Retry(Region),
    /// The user edited the composer.
    DraftChanged(String),
    /// The prompt request left.
    PromptSent,
    /// The server accepted the prompt with this text. The draft clears only while it still
    /// holds that text: what the user typed while the prompt was in flight stays.
    PromptAccepted(String),
    /// The prompt request failed; the draft stays.
    PromptFailed(String),
}

impl State {
    /// Applies one change. Pure: no I/O and no clock, so a replay gives the same state.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Event(event) => reduce_event(self, &event),
            Action::Connection(status) => self.connection = status,
            Action::SessionsLoaded(sessions) => self.load_sessions(&sessions),
            Action::SessionsFailed(message) => self.sidebar = LoadStatus::Failed { message },
            Action::SessionCreated(session) => self.add_created_session(&session),
            Action::CreateFailed(message) => self.create_error = Some(message),
            Action::OpenSession(id) => self.open_session(id, LoadStatus::Loading),
            Action::MessagesLoaded { session, messages } => {
                self.set_messages(&session, LoadStatus::Ready, Some(&messages));
            }
            Action::MessagesFailed { session, message } => {
                self.set_messages(&session, LoadStatus::Failed { message }, None);
            }
            Action::Retry(region) => self.retry(region),
            Action::DraftChanged(draft) => self.composer.draft = draft,
            Action::PromptSent => self.composer.status = ComposerStatus::Sending,
            Action::PromptAccepted(sent) => {
                if self.composer.draft == sent {
                    self.composer.draft.clear();
                }
                self.composer.status = ComposerStatus::Idle;
            }
            Action::PromptFailed(message) => {
                self.composer.status = ComposerStatus::Failed { message };
            }
        }
    }

    fn load_sessions(&mut self, sessions: &[Session]) {
        let entries = sessions
            .iter()
            .filter(|session| session.location.directory == self.directory)
            .map(session_entry)
            .collect();
        self.sessions.replace(entries);
        self.sidebar = LoadStatus::Ready;
    }

    fn add_created_session(&mut self, session: &Session) {
        self.create_error = None;
        let entry = session_entry(session);
        if session.location.directory == self.directory {
            self.sessions.insert(entry.clone());
        }
        if self.sidebar_admits(&session.location.directory) {
            self.global.list.insert(entry);
        }
        let id = SessionId::from(session.id.as_str());
        self.open_session(id, LoadStatus::Ready);
    }

    fn open_session(&mut self, id: SessionId, load: LoadStatus) {
        if self.open.as_ref().is_some_and(|open| open.id == id) {
            return;
        }
        self.open = Some(OpenSession {
            id,
            messages: Vec::new(),
            load,
        });
    }

    /// A loaded list replaces the messages; a failure keeps what is on screen under the error.
    fn set_messages(&mut self, session: &SessionId, load: LoadStatus, loaded: Option<&[Message]>) {
        let Some(open) = self.open.as_mut().filter(|open| open.id == *session) else {
            tracing::debug!(
                target: crate::logging::STATE,
                session = %session,
                "[STATE] Dropped messages of a session that is no longer open"
            );
            return;
        };
        if let Some(messages) = loaded {
            open.messages = chat_messages(messages);
        }
        open.load = load;
    }

    fn retry(&mut self, region: Region) {
        match region {
            Region::Sidebar => self.sidebar = LoadStatus::Loading,
            Region::Messages => {
                if let Some(open) = self.open.as_mut() {
                    open.load = LoadStatus::Loading;
                }
            }
            Region::Composer => self.composer.status = ComposerStatus::Idle,
            Region::NewSession => self.create_error = None,
        }
    }
}

#[cfg(test)]
#[path = "action_test.rs"]
mod action_test;
