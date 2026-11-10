//! The task that talks to one running server: auth check, the window's requests, the event
//! stream. It runs on the I/O runtime until the window drops its channels or aborts it.

use futures::channel::mpsc::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use oc_client::Client;
use oc_state::{Action, ConnectionStatus, ServerFailure};
use tokio::task::JoinSet;

use super::protocol::{Closed, Command, Update};
use super::reconnect::follow_events;
use super::{catalog, send, startup};

/// Checks auth, then serves `commands` and follows the global event stream. Requests run
/// side by side in a [`JoinSet`] this task owns, so they stop with it.
pub(crate) async fn run_session(
    client: Client,
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
                    let request = answer(client.clone(), command, updates.clone());
                    requests.spawn(request);
                }
                None => break,
            },
            Some(_) = requests.join_next() => {}
        }
    }
    requests.shutdown().await;
}

/// Runs one request and reports its answers.
async fn answer(client: Client, command: Command, mut updates: Sender<Update>) {
    if dispatch(&client, command, &mut updates).await == Err(Closed) {
        tracing::debug!(target: crate::logging::SERVER, "[SERVER] Dropped an answer with no window");
    }
}

async fn dispatch(
    client: &Client,
    command: Command,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    match command {
        Command::Startup { catalog } => startup::load_startup(client, &catalog, updates).await,
        Command::Refetch { catalogs, open } => {
            startup::refetch(client, catalogs, open, updates).await
        }
        Command::LoadMessages { session, directory } => {
            startup::load_messages(client, &session, &directory, updates).await
        }
        Command::LoadCatalog(directory) => catalog::load(client, &directory, updates).await,
        Command::LoadSessionRecord { session, directory } => {
            startup::load_record(client, &session, &directory, updates).await
        }
        Command::CreateSession(request) => {
            send::create(
                client,
                &request.target,
                request.agent.as_deref(),
                request.model.as_ref(),
                updates,
            )
            .await
        }
        Command::CreateAndPrompt(request) => {
            send::create_and_prompt(
                client,
                &request.target,
                &request.text,
                request.agent.as_deref(),
                request.model.as_ref(),
                updates,
            )
            .await
        }
        Command::SendPrompt(request) => send::prompt(client, request.open(), updates).await,
    }
}

#[cfg(test)]
#[path = "session_test.rs"]
mod session_test;
