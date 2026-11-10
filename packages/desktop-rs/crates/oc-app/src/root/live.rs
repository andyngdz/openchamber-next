//! The root view's side of the live server: intents become requests, updates become actions.

use futures::StreamExt;
use gpui_kit::{Context, Window};
use oc_state::{Action, ConnectionStatus, Region, ServerFailure};

use super::RootView;
use crate::live::{Command, Update};
use crate::views::Intent;

impl RootView {
    /// Starts the session against the server started before the window opened, or shows why
    /// it did not start.
    pub(crate) fn begin(
        &mut self,
        started: Result<u16, ServerFailure>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match started {
            Ok(port) => self.connect(port, window, cx),
            Err(failure) => {
                self.dispatch(Action::Connection(ConnectionStatus::Failed(failure)), cx)
            }
        }
    }

    /// Does what a control asked for.
    pub(crate) fn handle(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
        match intent {
            Intent::Apply(action) => {
                self.request_for(&action, cx);
                self.dispatch(action, cx);
            }
            Intent::NewSession => {
                if *self.state.read(cx).connection() == ConnectionStatus::Connected {
                    self.send(Command::CreateSession).ok();
                }
            }
            Intent::SendPrompt => self.send_prompt(cx),
            Intent::RestartServer => self.restart_server(window, cx),
        }
    }

    /// Opening a session loads its messages; Retry sends the region's request again.
    fn request_for(&mut self, action: &Action, cx: &Context<Self>) {
        let open = self.state.read(cx).open().map(|open| open.id.clone());
        let command = match action {
            Action::OpenSession(id) => {
                (open.as_ref() != Some(id)).then(|| Command::LoadMessages(id.clone()))
            }
            Action::Retry(Region::Sidebar) => Some(Command::LoadSessions),
            Action::Retry(Region::Messages) => open.map(Command::LoadMessages),
            Action::Retry(Region::Composer)
            | Action::Event(_)
            | Action::Connection(_)
            | Action::SessionsLoaded(_)
            | Action::SessionsFailed(_)
            | Action::SessionCreated(_)
            | Action::MessagesLoaded { .. }
            | Action::MessagesFailed { .. }
            | Action::DraftChanged(_)
            | Action::PromptSent
            | Action::PromptAccepted
            | Action::PromptFailed(_) => None,
        };
        if let Some(command) = command {
            self.send(command).ok();
        }
    }

    fn send_prompt(&mut self, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        let Some(open) = state.open().filter(|_| state.can_send()) else {
            return;
        };
        let command = Command::SendPrompt {
            session: open.id.clone(),
            text: state.composer().draft.clone(),
        };
        self.dispatch(Action::PromptSent, cx);
        if self.send(command).is_err() {
            let failed = Action::PromptFailed("Not connected to the server".to_owned());
            self.dispatch(failed, cx);
        }
    }

    /// Stops the server, shows Starting, and starts it again once that frame is drawn, since
    /// the start blocks the main thread until the port line arrives.
    fn restart_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        link.disconnect();
        link.process().shutdown();
        self.dispatch(Action::Connection(ConnectionStatus::Starting), cx);
        let view = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            view.update(cx, |this, cx| this.start_server(window, cx))
                .ok();
        });
    }

    fn start_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        let started = link.process().start();
        self.begin(started, window, cx);
    }

    fn connect(&mut self, port: u16, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        let read_updates = |mut updates: futures::channel::mpsc::Receiver<Update>| {
            cx.spawn_in(window, async move |view, cx| {
                while let Some(update) = updates.next().await {
                    let applied = view.update_in(cx, |this, window, cx| {
                        this.on_update(update, window, cx);
                    });
                    if applied.is_err() {
                        break;
                    }
                }
            })
        };
        if let Err(error) = link.connect(port, read_updates) {
            let failure = ServerFailure {
                message: format!("Could not create the server client: {error}"),
                ..ServerFailure::default()
            };
            self.dispatch(Action::Connection(ConnectionStatus::Failed(failure)), cx);
        }
    }

    fn on_update(&mut self, update: Update, window: &mut Window, cx: &mut Context<Self>) {
        match update {
            Update::Authenticated | Update::Reconnected => {
                self.record_opencode_pids();
                self.refetch(cx);
            }
            Update::Apply(action) => self.apply_live(action, window, cx),
        }
    }

    /// A dropped stream or a failed check may mean the CLI died: then the session ends and the
    /// header shows how the server exited.
    fn apply_live(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        if let Action::Connection(status) = &action {
            match status {
                ConnectionStatus::Connected => self.record_opencode_pids(),
                ConnectionStatus::Reconnecting { .. } | ConnectionStatus::Failed(_) => {
                    if let Some(exit) = self.server_exit() {
                        self.dispatch(Action::Connection(ConnectionStatus::ServerExited(exit)), cx);
                        return;
                    }
                }
                ConnectionStatus::Starting | ConnectionStatus::ServerExited(_) => {}
            }
        }
        if action == Action::PromptAccepted {
            self.draft
                .update(cx, |draft, cx| draft.set_value("", window, cx));
        }
        self.dispatch(action, cx);
    }

    /// How the CLI ended, once it has; the session with it is dropped.
    fn server_exit(&mut self) -> Option<oc_state::ServerExit> {
        let link = self.link.as_mut()?;
        let exit = link.process().exit()?;
        link.disconnect();
        Some(exit)
    }

    fn record_opencode_pids(&self) {
        if let Some(link) = self.link.as_ref() {
            link.process().record_opencode_pids();
        }
    }

    /// Reloads the session list and the open session's messages.
    fn refetch(&mut self, cx: &Context<Self>) {
        let open = self.state.read(cx).open().map(|open| open.id.clone());
        let requests = [Some(Command::LoadSessions), open.map(Command::LoadMessages)];
        for command in requests.into_iter().flatten() {
            self.send(command).ok();
        }
    }

    fn send(&mut self, command: Command) -> Result<(), Command> {
        match self.link.as_mut() {
            Some(link) => link.send(command),
            None => Err(command),
        }
    }
}
