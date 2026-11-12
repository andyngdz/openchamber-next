//! What the window and the I/O runtime say to each other.
//!
//! The window turns a control into a [`Command`]; the server session answers with an
//! [`Update`], which the window applies through the same `State` methods a fixture replay
//! uses. The settings writer has its own channel and [`SettingsCommand`].

use futures::SinkExt;
use futures::channel::mpsc::Sender;
use oc_client::SettingsPatch;
use oc_client::control::ControlEvent;
use oc_client::form::FormInfo;
use oc_client::permission::PermissionRequest;
use oc_client::queue::{MessageQueue, QueueUpdate};
use oc_client::routing::{AutoAcceptSnapshot, RoutingState};
use oc_client::types::{ModelRef, Session};
use oc_state::{Action, SelectionAction, SessionId, SidebarAction};

use super::execution::{DismissCall, SendExecution};

/// Where a new session goes: one project's directory, or a Chat folder the window minted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateTarget {
    /// A session in an existing project directory.
    Project {
        /// Absolute project directory.
        directory: String,
    },
    /// A Chat session in a folder the window chose and the server must create.
    Chat {
        /// `<chatsRoot>/YYYY-MM-DD/session-<uuid v4>`.
        folder: String,
    },
}

impl CreateTarget {
    /// The directory the session is created in.
    pub(crate) fn directory(&self) -> &str {
        match self {
            Self::Project { directory } => directory,
            Self::Chat { folder } => folder,
        }
    }
}

/// The open session whose messages a refetch reloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenMessages {
    /// The session id.
    pub(crate) session: SessionId,
    /// The directory it lives in.
    pub(crate) directory: String,
}

/// A request the window sends to the server session task.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Command {
    /// Load the settings, the home info, the global list and the draft's catalog, in order.
    Startup {
        /// The directory the open draft reads its catalog from.
        catalog: String,
    },
    /// Re-read the settings, the global list, every loaded catalog and the open messages.
    Refetch {
        /// Directories whose catalogs are loaded.
        catalogs: Vec<String>,
        /// The open session's messages, when one is open.
        open: Option<OpenMessages>,
    },
    /// `GET /api/session/:id/message`.
    LoadMessages {
        /// The session.
        session: SessionId,
        /// The directory it lives in.
        directory: String,
    },
    /// The config, provider, model and agent calls for one directory.
    LoadCatalog(String),
    /// `GET /api/session/:id`: the record the selection starts from.
    LoadSessionRecord {
        /// The session.
        session: SessionId,
        /// The directory it lives in.
        directory: String,
    },
    /// `POST /api/session` and open it; the sidebar's New session.
    CreateSession(Box<CreateSessionRequest>),
    /// `POST /api/session`, then the prompt, for a send from an empty draft.
    CreateAndPrompt(Box<CreateDraftPrompt>),
    /// Run one composer send's plan: its dismissals, then the queue take and the prompt.
    Send(Box<SendExecution>),
    /// Re-read the queue, the auto-accept snapshot and the routing state.
    RefreshControl,
    /// Re-read one directory's pending permissions and forms.
    LoadRequests(String),
}

/// A session to create and open, with the draft's picks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateSessionRequest {
    /// Where the session goes.
    pub(crate) target: CreateTarget,
    /// The agent the draft picked.
    pub(crate) agent: Option<String>,
    /// The model the draft picked.
    pub(crate) model: Option<ModelRef>,
}

/// A session to create from an empty draft, and its first prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateDraftPrompt {
    /// Where the session goes.
    pub(crate) target: CreateTarget,
    /// The composer text.
    pub(crate) text: String,
    /// The agent the draft picked.
    pub(crate) agent: Option<String>,
    /// The model the draft picked.
    pub(crate) model: Option<ModelRef>,
}

/// A settings write queued for the writer task.
#[derive(Debug)]
pub(crate) enum SettingsCommand {
    /// Merge this partial document into the pending write.
    Write(Box<SettingsPatch>),
    /// Write everything pending now and answer on the channel, bounded by the caller's wait.
    Flush(std::sync::mpsc::Sender<()>),
}

/// What the server session task reports to the window.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Update {
    /// A change the window applies through [`oc_state::State::apply`].
    Apply(Action),
    /// A sidebar input change, applied through [`oc_state::State::apply_sidebar`].
    Sidebar(SidebarAction),
    /// A selection change, applied through [`oc_state::State::apply_selection`].
    Selection(SelectionAction),
    /// The auth check passed; the window records OpenCode's pids and loads its lists.
    Authenticated,
    /// The event stream is back after a drop; events of the gap are lost, so the window
    /// refetches its lists, and records OpenCode's pids, since the server may have restarted it.
    Reconnected,
    /// An open session accepted a prompt with this text.
    Sent {
        /// What was sent; the draft clears only while it still holds it.
        text: String,
    },
    /// A session was created and opens now.
    Created {
        /// The new session.
        session: Session,
        /// The directory it was created in.
        directory: String,
    },
    /// A session was created from a draft, and its first prompt was accepted.
    CreatedAndSent {
        /// The new session.
        session: Session,
        /// The directory it was created in.
        directory: String,
        /// What was sent.
        text: String,
    },
    /// One frame of `/api/openchamber/events`.
    Control(ControlEvent),
    /// The server answered 404 for the control stream: no queue strip and no shield modes.
    ControlStreamMissing,
    /// `GET /api/permission/request` answered or failed for one directory.
    Permissions {
        /// The directory the list was read from.
        directory: String,
        /// The list, or the error text that keeps the previous entries.
        result: Result<Vec<PermissionRequest>, String>,
    },
    /// `GET /api/form` answered or failed for one directory.
    Forms {
        /// The directory the list was read from.
        directory: String,
        /// The list, or the error text that keeps the previous entries.
        result: Result<Vec<FormInfo>, String>,
    },
    /// `GET /api/message-queue` answered or failed.
    QueueLoaded(Result<MessageQueue, String>),
    /// `GET /api/permission-auto-accept` answered or failed.
    AutoAcceptLoaded(Result<AutoAcceptSnapshot, String>),
    /// `GET /api/routing` answered or failed.
    RoutingLoaded(Result<RoutingState, String>),
    /// One session's queue after an enqueue.
    QueueUpdated(Box<QueueUpdate>),
    /// The composer text was queued; the draft clears while it still holds it.
    Queued {
        /// What was queued.
        text: String,
    },
    /// The dismissal step stopped the send with nothing to queue; the composer is idle again.
    SendStopped,
    /// A dismissal failed for a reason other than 404; the request goes back where it was.
    DismissFailed {
        /// The request to restore.
        call: Box<DismissCall>,
        /// Why the call failed, for the log.
        message: String,
    },
}

/// The window stopped listening for updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Closed;

/// Sends one update; [`Closed`] once the window's reader is gone.
pub(crate) async fn emit(updates: &mut Sender<Update>, update: Update) -> Result<(), Closed> {
    updates.send(update).await.map_err(|_| Closed)
}

/// Sends one state action.
pub(crate) async fn apply(updates: &mut Sender<Update>, action: Action) -> Result<(), Closed> {
    emit(updates, Update::Apply(action)).await
}

/// Sends one sidebar action.
pub(crate) async fn sidebar(
    updates: &mut Sender<Update>,
    action: SidebarAction,
) -> Result<(), Closed> {
    emit(updates, Update::Sidebar(action)).await
}

/// Sends one selection action.
pub(crate) async fn selection(
    updates: &mut Sender<Update>,
    action: SelectionAction,
) -> Result<(), Closed> {
    emit(updates, Update::Selection(action)).await
}
