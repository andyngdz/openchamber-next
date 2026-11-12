//! What an intent turns into: session loads, catalog loads, creation and the send sequence.

use gpui_kit::Context;
use oc_state::{Action, ComposerStatus, ConnectionStatus, Region, SendKey, SessionId, State};

use super::RootView;
use crate::live::{
    Command, CreateDraftPrompt, CreateSessionRequest, DismissCall, OpenMessages, SendExecution,
};

const NOT_CONNECTED: &str = "Not connected to the server";
const CHAT_FOLDER_UNKNOWN: &str = "The chat folder is not known yet; retry once the server answers";

/// What one send turns into: nothing, the draft's session, or the open session's plan.
enum Flow {
    Nothing,
    Draft(CreateDraftPrompt),
    Send(Box<SendExecution>),
    Failed(String),
}

impl RootView {
    /// Opening a session loads its messages, record and catalog; Retry sends the region's
    /// request again.
    pub(super) fn request_for(&mut self, action: &Action, cx: &mut Context<Self>) {
        match action {
            Action::OpenSession(id) => {
                let open = { self.state.read(cx).open().map(|session| session.id.clone()) };
                if open.as_ref() == Some(id) {
                    return;
                }
                let directory = self.session_directory(id, cx);
                let request = Command::LoadMessages {
                    session: id.clone(),
                    directory: directory.clone(),
                };
                self.send(request).ok();
                self.request_session(&directory, cx);
            }
            Action::Retry(Region::Sidebar) => self.startup(cx),
            Action::Retry(Region::Messages) => {
                if let Some(open) = self.open_messages(cx)
                    && self
                        .send(Command::LoadMessages {
                            session: open.session,
                            directory: open.directory,
                        })
                        .is_err()
                {
                    tracing::debug!(target: crate::logging::SERVER, "[SERVER] Dropped a message load with no server session");
                }
            }
            Action::Retry(Region::NewSession) => self.create_session(cx),
            Action::Retry(Region::Composer)
            | Action::Event(_)
            | Action::Connection(_)
            | Action::SessionsLoaded(_)
            | Action::SessionsFailed(_)
            | Action::SessionCreated(_)
            | Action::CreateFailed(_)
            | Action::MessagesLoaded { .. }
            | Action::MessagesFailed { .. }
            | Action::DraftChanged(_)
            | Action::PromptSent
            | Action::PromptAccepted(_)
            | Action::PromptFailed(_) => {}
        }
    }

    /// Creates a session for the draft and opens it; the sidebar's New session.
    pub(super) fn create_session(&mut self, cx: &mut Context<Self>) {
        let plan = {
            let state = self.state.read(cx);
            if *state.connection() != ConnectionStatus::Connected {
                return;
            }
            let (agent, model) = super::plan::selection_parts(state.current_selection());
            match super::plan::create_target(state) {
                Some(target) => Ok(Command::CreateSession(Box::new(CreateSessionRequest {
                    target,
                    agent,
                    model,
                }))),
                None => Err(CHAT_FOLDER_UNKNOWN.to_owned()),
            }
        };
        match plan {
            Ok(command) => {
                self.send(command).ok();
            }
            Err(message) => self.dispatch(Action::CreateFailed(message), cx),
        }
    }

    /// Sends from an open session, or creates the draft's session and sends its first prompt.
    pub(super) fn send_prompt(&mut self, cx: &mut Context<Self>) {
        self.send_with_key(SendKey::Plain, cx);
    }

    /// Runs the send plan for `key` (`ChatInput.tsx:2101-2113`): create the draft's session,
    /// queue the text, or dismiss the tree's requests and then queue or send.
    pub(super) fn send_with_key(&mut self, key: SendKey, cx: &mut Context<Self>) {
        let flow = {
            let state = self.state.read(cx);
            if *state.connection() != ConnectionStatus::Connected
                || state.composer().status == ComposerStatus::Sending
            {
                return;
            }
            if state.open().is_none() {
                self.draft_flow(state)
            } else {
                // Without a control stream a busy session still takes a plain prompt.
                let key = if self.control_stream_missing() {
                    SendKey::Command
                } else {
                    key
                };
                let plan = state.send_plan(key);
                let (agent, model) = super::plan::selection_parts(state.current_selection());
                match crate::live::from_plan(&plan, state, agent, model) {
                    Some(execution) => Flow::Send(Box::new(execution)),
                    None => Flow::Nothing,
                }
            }
        };
        match flow {
            Flow::Nothing => {}
            Flow::Draft(request) => {
                self.dispatch(Action::PromptSent, cx);
                if self
                    .send(Command::CreateAndPrompt(Box::new(request)))
                    .is_err()
                {
                    self.dispatch(Action::PromptFailed(NOT_CONNECTED.to_owned()), cx);
                }
            }
            Flow::Send(execution) => {
                self.dispatch(Action::PromptSent, cx);
                let dismissals = execution.dismiss.clone();
                if self.send(Command::Send(execution)).is_err() {
                    self.dispatch(Action::PromptFailed(NOT_CONNECTED.to_owned()), cx);
                } else {
                    // The dock empties now; a failed call puts its request back.
                    for call in &dismissals {
                        self.dismiss_locally(call, cx);
                    }
                }
            }
            Flow::Failed(message) => self.dispatch(Action::PromptFailed(message), cx),
        }
    }

    /// The send from an empty draft: create its session and prompt.
    fn draft_flow(&self, state: &State) -> Flow {
        let draft = state.composer().draft.clone();
        if draft.trim().is_empty() {
            return Flow::Nothing;
        }
        let (agent, model) = super::plan::selection_parts(state.current_selection());
        match super::plan::create_target(state) {
            Some(target) => Flow::Draft(CreateDraftPrompt {
                target,
                text: draft,
                agent,
                model,
            }),
            None => Flow::Failed(CHAT_FOLDER_UNKNOWN.to_owned()),
        }
    }

    /// Takes a dismissed request off screen before its call answers.
    fn dismiss_locally(&mut self, call: &DismissCall, cx: &mut Context<Self>) {
        if let Some(event) = crate::live::settled_event(call) {
            self.dispatch(Action::Event(event), cx);
        }
    }

    /// Puts a request back after its dismissal failed for a reason other than 404.
    pub(super) fn restore_dismissed(&mut self, call: &DismissCall, cx: &mut Context<Self>) {
        if let Some(event) = crate::live::restored_event(call) {
            self.dispatch(Action::Event(event), cx);
        }
    }

    /// Loads the settings, home info, global list and draft catalog.
    pub(super) fn startup(&mut self, cx: &Context<Self>) {
        let catalog = self.state.read(cx).directory().to_owned();
        self.catalogs.insert(catalog.clone());
        self.send(Command::Startup { catalog }).ok();
    }

    /// Reloads everything a reconnect may have missed.
    pub(super) fn refetch(&mut self, cx: &Context<Self>) {
        let catalogs: Vec<String> = self.catalogs.iter().cloned().collect();
        let open = self.open_messages(cx);
        self.send(Command::Refetch { catalogs, open }).ok();
    }

    pub(super) fn open_messages(&self, cx: &Context<Self>) -> Option<OpenMessages> {
        let state = self.state.read(cx);
        let open = state.open()?;
        let directory = state
            .open_entry()
            .map(|entry| entry.directory.clone())
            .unwrap_or_else(|| state.directory().to_owned());
        Some(OpenMessages {
            session: open.id.clone(),
            directory,
        })
    }

    /// Loads a directory's catalog once and the open session's record.
    pub(super) fn request_session(&mut self, directory: &str, cx: &Context<Self>) {
        if self.catalogs.insert(directory.to_owned()) {
            self.send(Command::LoadCatalog(directory.to_owned())).ok();
        }
        let session = { self.state.read(cx).open().map(|open| open.id.clone()) };
        if let Some(session) = session {
            let request = Command::LoadSessionRecord {
                session,
                directory: directory.to_owned(),
            };
            self.send(request).ok();
        }
    }

    /// The directory a session lives in, from either list, else the app's own.
    fn session_directory(&self, id: &SessionId, cx: &Context<Self>) -> String {
        let state = self.state.read(cx);
        state
            .sessions()
            .get(id)
            .or_else(|| state.global_sessions().get(id))
            .map(|entry| entry.directory.clone())
            .filter(|directory| !directory.is_empty())
            .unwrap_or_else(|| state.directory().to_owned())
    }
}
