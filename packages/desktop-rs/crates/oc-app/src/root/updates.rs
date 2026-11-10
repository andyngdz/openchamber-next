//! Applying what the server session sends: state actions, sidebar inputs, selection changes
//! and the answers of the send sequence.

use gpui_kit::{Context, Window};
use oc_state::{Action, ConnectionStatus, SelectionAction, SidebarAction};

use super::RootView;
use crate::live::Update;

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
                self.dispatch(Action::PromptAccepted(text), cx);
                self.apply_selection(SelectionAction::SendSucceeded, cx);
            }
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
                self.dispatch(Action::PromptAccepted(text), cx);
                self.request_session(&directory, cx);
            }
        }
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
        // Text typed while the prompt was in flight stays in the composer.
        if let Action::PromptAccepted(sent) = &action
            && self.draft.read(cx).value() == sent.as_str()
        {
            self.draft
                .update(cx, |draft, cx| draft.set_value("", window, cx));
        }
        self.dispatch(action, cx);
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

    /// Sends the settings patch a change queued, then repaints.
    pub(super) fn after_change(&mut self, cx: &mut Context<Self>) {
        let patch = self
            .state
            .update(cx, |state, _| state.take_settings_patch());
        if let Some(patch) = patch
            && let Some(link) = self.link.as_ref()
        {
            link.write_settings(patch);
        }
        cx.notify();
    }
}
