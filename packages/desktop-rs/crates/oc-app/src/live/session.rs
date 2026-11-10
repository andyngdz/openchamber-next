//! The task that talks to one running server: auth check, the window's requests, the event
//! stream. It runs on the I/O runtime until the window drops its channels or aborts it.

use futures::channel::mpsc::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use oc_client::{Client, Directory};
use oc_state::{Action, ConnectionStatus, ServerFailure};
use tokio::task::JoinSet;

use super::protocol::{Command, Update};
use super::reconnect::follow_events;

/// Checks auth, then serves `commands` and follows the global event stream. Requests run
/// side by side in a [`JoinSet`] this task owns, so they stop with it.
pub(crate) async fn run_session(
    client: Client,
    directory: Directory,
    mut commands: Receiver<Command>,
    mut updates: Sender<Update>,
) {
    if let Err(error) = client.check_auth().await {
        tracing::warn!(target: crate::logging::SERVER, %error, "[SERVER] Auth check failed");
        let failure = ServerFailure {
            message: format!("Server check failed: {error}"),
            ..ServerFailure::default()
        };
        let failed = Update::Apply(Action::Connection(ConnectionStatus::Failed(failure)));
        updates.send(failed).await.ok();
        return;
    }
    if updates.send(Update::Authenticated).await.is_err() {
        return;
    }
    let events = follow_events(|| client.events(), tokio::time::sleep, updates.clone());
    tokio::pin!(events);
    let mut requests = JoinSet::new();
    loop {
        tokio::select! {
            () = &mut events => break,
            command = commands.next() => match command {
                Some(command) => {
                    let request = answer(client.clone(), directory.clone(), command, updates.clone());
                    requests.spawn(request);
                }
                None => break,
            },
            Some(_) = requests.join_next() => {}
        }
    }
    requests.shutdown().await;
}

/// Sends one request and reports its answer as an action.
async fn answer(
    client: Client,
    directory: Directory,
    command: Command,
    mut updates: Sender<Update>,
) {
    let action = match command {
        Command::LoadSessions => match client.list_sessions(&directory).await {
            Ok(sessions) => Action::SessionsLoaded(sessions),
            Err(error) => Action::SessionsFailed(format!("Could not load sessions: {error}")),
        },
        Command::LoadMessages(session) => {
            match client.list_messages(&directory, session.as_str()).await {
                Ok(messages) => Action::MessagesLoaded { session, messages },
                Err(error) => Action::MessagesFailed {
                    session,
                    message: format!("Could not load messages: {error}"),
                },
            }
        }
        Command::CreateSession => match client.create_session(&directory, None, None).await {
            Ok(session) => Action::SessionCreated(session),
            Err(error) => Action::CreateFailed(format!("Could not create a session: {error}")),
        },
        Command::SendPrompt { session, text } => {
            match client
                .send_prompt(&directory, session.as_str(), &text)
                .await
            {
                Ok(_) => Action::PromptAccepted(text),
                Err(error) => Action::PromptFailed(format!("Could not send the prompt: {error}")),
            }
        }
    };
    updates.send(Update::Apply(action)).await.ok();
}

#[cfg(test)]
#[path = "session_test.rs"]
mod session_test;
