//! The root view's live connection: waiting for the sidecar, connecting to it, and
//! restarting it or noticing that it exited.

use futures::StreamExt;
use futures::channel::oneshot;
use gpui_kit::{Context, Window};
use oc_state::{Action, ConnectionStatus, ServerFailure};

use super::RootView;
use crate::live::{Command, Update};
use crate::views::Intent;

/// The server thread's answer to a start.
pub(crate) type Started = oneshot::Receiver<Result<u16, ServerFailure>>;

impl RootView {
    /// Does what a control asked for.
    pub(crate) fn handle(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
        match intent {
            Intent::Apply(action) => {
                self.request_for(&action, cx);
                self.dispatch(action, cx);
            }
            Intent::NewSession => self.create_session(cx),
            Intent::SendPrompt => self.send_prompt(cx),
            Intent::RestartServer => self.restart_server(window, cx),
        }
    }

    /// Waits off the main thread for the server thread to start the sidecar, then begins.
    pub(crate) fn await_start(
        &mut self,
        started: Started,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.exit_check = None;
        self.starting = Some(cx.spawn_in(window, async move |view, cx| {
            let started = started.await.unwrap_or_else(|_| {
                Err(ServerFailure {
                    message: "The server thread stopped before the server started".to_owned(),
                    ..ServerFailure::default()
                })
            });
            view.update_in(cx, |this, window, cx| this.begin(started, window, cx))
                .ok();
        }));
    }

    /// Starts the session against the started server, or shows why it did not start.
    fn begin(
        &mut self,
        started: Result<u16, ServerFailure>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match started {
            Ok(port) => {
                self.record_pids_on_list = true;
                self.connect(port, window, cx);
            }
            Err(failure) => {
                self.dispatch(Action::Connection(ConnectionStatus::Failed(failure)), cx)
            }
        }
    }

    /// Drops the session, shows Starting, and has the server thread stop the server and start
    /// it again; the window stays responsive meanwhile.
    fn restart_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        link.disconnect();
        let started = link.process().start();
        self.dispatch(Action::Connection(ConnectionStatus::Starting), cx);
        self.await_start(started, window, cx);
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

    /// Once the server thread says the CLI ended, the session with it is dropped and the
    /// header shows the exit. The status that prompted the check shows until the answer.
    pub(super) fn check_server_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        let answer = link.process().exit();
        self.exit_check = Some(cx.spawn_in(window, async move |view, cx| {
            let Ok(Some(exit)) = answer.await else {
                return;
            };
            view.update_in(cx, |this, _, cx| {
                if let Some(link) = this.link.as_mut() {
                    link.disconnect();
                }
                this.dispatch(Action::Connection(ConnectionStatus::ServerExited(exit)), cx);
            })
            .ok();
        }));
    }

    pub(super) fn record_opencode_pids(&self) {
        if let Some(link) = self.link.as_ref() {
            link.process().record_opencode_pids();
        }
    }

    pub(super) fn send(&mut self, command: Command) -> Result<(), Command> {
        match self.link.as_mut() {
            Some(link) => link.send(command),
            None => Err(command),
        }
    }
}
