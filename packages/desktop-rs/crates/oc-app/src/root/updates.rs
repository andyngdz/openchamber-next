//! Applying what the server session sends: state actions, sidebar inputs, selection changes
//! and the answers of the send sequence.

use gpui_kit::{Context, Window};
use oc_client::control::ControlEvent;
use oc_state::{Action, ConnectionStatus, SelectionAction, SidebarAction};

use super::RootView;
use crate::live::{Command, DismissCall, ServerLink, Update, WriteOutcome};

impl RootView {
    pub(super) fn on_update(
        &mut self,
        update: Update,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match update {
            Update::Apply(action) => self.apply_live(action, window, cx),
            Update::Sidebar(action) => self.apply_sidebar(action, cx),
            Update::Selection(action) => self.apply_selection(action, cx),
            Update::Authenticated => {
                self.record_opencode_pids();
                self.startup(cx);
            }
            Update::Reconnected => {
                self.record_opencode_pids();
                self.refetch(cx);
            }
            Update::Sent { text } => {
                self.accept_prompt(text, window, cx);
                self.apply_selection(SelectionAction::SendSucceeded, cx);
            }
            Update::Queued { text } => self.accept_prompt(text, window, cx),
            Update::SendStopped => self.dispatch(Action::PromptAccepted(String::new()), cx),
            Update::Created { session, directory } => {
                self.dispatch(Action::SessionCreated(session), cx);
                self.request_session(&directory, cx);
            }
            Update::CreatedAndSent {
                session,
                directory,
                text,
            } => {
                // The draft's selection is the one the create used; it is still current
                // until the session opens.
                self.apply_selection(SelectionAction::SendSucceeded, cx);
                self.dispatch(Action::SessionCreated(session), cx);
                self.accept_prompt(text, window, cx);
                self.request_session(&directory, cx);
            }
            Update::Control(event) => self.apply_control(event, cx),
            Update::ControlStreamMissing => {
                if let Some(link) = self.link.as_mut() {
                    link.mark_control_missing();
                }
            }
            Update::Permissions { directory, result } => {
                self.state.update(cx, |state, _| match result {
                    Ok(requests) => state.permissions_loaded(&directory, requests),
                    Err(message) => state.permissions_failed(&directory, &message),
                });
                cx.notify();
            }
            Update::Forms { directory, result } => {
                self.state.update(cx, |state, _| match result {
                    Ok(forms) => state.forms_loaded(&directory, forms),
                    Err(message) => state.forms_failed(&directory, &message),
                });
                cx.notify();
            }
            Update::QueueLoaded(result) => {
                self.state.update(cx, |state, _| match result {
                    Ok(queue) => state.queue_loaded(queue),
                    Err(message) => tracing::warn!(
                        target: crate::logging::SERVER,
                        %message,
                        "[SERVER] Message queue read kept its last state"
                    ),
                });
                cx.notify();
            }
            Update::AutoAcceptLoaded(result) => {
                self.state.update(cx, |state, _| match result {
                    Ok(snapshot) => state.auto_accept_loaded(snapshot),
                    Err(message) => state.auto_accept_failed(&message),
                });
                cx.notify();
            }
            Update::RoutingLoaded(result) => {
                self.state.update(cx, |state, _| match result {
                    Ok(routing) => state.routing_loaded(routing),
                    Err(message) => state.routing_failed(&message),
                });
                cx.notify();
            }
            Update::QueueUpdated(update) => {
                self.state
                    .update(cx, |state, _| state.queue_updated(&update));
                cx.notify();
            }
            Update::DismissFailed { call, message } => {
                self.apply_restore(*call, &message, cx);
            }
        }
    }

    /// Whether the server has no control stream. The window then offers no queue strip and no
    /// shield modes, and a send is a plain prompt even while a turn runs.
    pub(super) fn control_stream_missing(&self) -> bool {
        self.link
            .as_ref()
            .is_some_and(ServerLink::control_stream_missing)
    }

    /// One control frame: the ready frame re-reads the snapshots, every other frame changes
    /// the state it names.
    fn apply_control(&mut self, event: ControlEvent, cx: &mut Context<Self>) {
        if matches!(event, ControlEvent::Ready) {
            self.send(Command::RefreshControl).ok();
        }
        self.state
            .update(cx, |state, _| state.control_event(&event));
        cx.notify();
    }

    /// Puts a request back after its dismissal failed, and logs why. The record travels with
    /// the answer, so the request returns exactly as it was.
    fn apply_restore(&mut self, call: DismissCall, message: &str, cx: &mut Context<Self>) {
        tracing::warn!(
            target: crate::logging::APP,
            session = %call.session_id,
            request = %call.id,
            %message,
            "[APP] Restored a pending request its dismissal did not answer"
        );
        self.restore_dismissed(&call, cx);
        // The record puts it back at once; the list is authoritative, so a request the server
        // may hide from a fresh event still returns.
        self.send(Command::LoadRequests(call.directory)).ok();
    }

    /// A dropped stream or a failed check may mean the CLI died: the server thread is asked,
    /// and if it did, the session ends and the header shows how the server exited.
    fn apply_live(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        if let Action::Connection(status) = &action {
            match status {
                ConnectionStatus::Connected => self.record_opencode_pids(),
                ConnectionStatus::Reconnecting { .. } | ConnectionStatus::Failed(_) => {
                    self.check_server_exit(window, cx);
                }
                ConnectionStatus::Starting | ConnectionStatus::ServerExited(_) => {}
            }
        }
        if matches!(action, Action::SessionsLoaded(_)) && self.record_pids_on_list {
            self.record_pids_on_list = false;
            self.record_opencode_pids();
        }
        match action {
            Action::PromptAccepted(text) => self.accept_prompt(text, window, cx),
            action => self.dispatch(action, cx),
        }
    }

    /// Applies an accepted prompt. The composer clears only while it still holds the text that
    /// was sent, so what the user typed while the prompt was in flight stays. Every path that
    /// accepts a prompt goes through here: the textarea is the view's, and `State::apply` only
    /// clears the state's copy of the draft.
    fn accept_prompt(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.draft.read(cx).value() == text.as_str() {
            self.draft
                .update(cx, |draft, cx| draft.set_value("", window, cx));
        }
        self.dispatch(Action::PromptAccepted(text), cx);
    }

    /// Applies a sidebar input change; the first list page records OpenCode's pids too.
    fn apply_sidebar(&mut self, action: SidebarAction, cx: &mut Context<Self>) {
        if matches!(action, SidebarAction::PageLoaded(_)) && self.record_pids_on_list {
            self.record_pids_on_list = false;
            self.record_opencode_pids();
        }
        self.state
            .update(cx, |state, _| state.apply_sidebar(action));
        self.after_change(cx);
    }

    /// Applies a selection change; its pick may have queued a settings write.
    pub(super) fn apply_selection(&mut self, action: SelectionAction, cx: &mut Context<Self>) {
        self.state
            .update(cx, |state, _| state.apply_selection(action));
        self.after_change(cx);
    }

    /// Sends the settings patch a change queued, then repaints. A write the live queue
    /// abandons has its keys cleared from the overlay, so the next read is authoritative.
    pub(super) fn after_change(&mut self, cx: &mut Context<Self>) {
        let patch = self
            .state
            .update(cx, |state, _| state.take_settings_patch());
        if let Some(patch) = patch {
            let dropped = self
                .link
                .as_ref()
                .is_some_and(|link| link.write_settings(patch.clone()) == WriteOutcome::Dropped);
            if dropped {
                self.state
                    .update(cx, |state, _| state.abandon_settings(&patch));
            }
        }
        cx.notify();
    }
}
