//! The settings writer: merges partial writes, waits out the debounce, `PUT`s them, and
//! replaces the state's settings with the server's answer.
//!
//! React debounces writes 200 ms (`persistence.ts:399`) and a collapse 700 ms
//! (`useSessionProjectViewState.ts:79-103`). A failed write keeps the choice locally, logs it,
//! and the next write carries it again; the window flushes what is pending when it closes.
//!
//! A flush answers its caller before it reports the write's answer to the window: the window
//! waits on that answer while closing, and the update channel may be full because its reader
//! has stopped. While closing, the answer is sent without waiting at all.

use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender};
use oc_client::{Client, Settings, SettingsPatch};
use oc_state::SidebarAction;

use super::protocol::{SettingsCommand, Update, sidebar};

/// React's settings debounce (`SETTINGS_DEBOUNCE_MS`, `persistence.ts:399`).
pub(crate) const SETTINGS_DEBOUNCE: Duration = Duration::from_millis(200);
/// React's collapse debounce (`useSessionProjectViewState.ts:79-103`).
pub(crate) const COLLAPSE_DEBOUNCE: Duration = Duration::from_millis(700);
/// One write's own limit, so a stuck request never holds the queue for the client's 15 s.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// Merges writes and sends them until the channel closes.
pub(crate) async fn run_settings(
    client: Client,
    mut commands: Receiver<SettingsCommand>,
    mut updates: Sender<Update>,
) {
    let mut pending: Option<SettingsPatch> = None;
    'outer: while let Some(command) = commands.next().await {
        match command {
            SettingsCommand::Write(patch) => pending = Some(merge(pending.take(), *patch)),
            SettingsCommand::Flush(done) => {
                let answer = write(&client, &mut pending).await;
                done.send(()).ok();
                report(&mut updates, answer).await;
                continue;
            }
        }
        let Some(batch) = pending.take() else {
            continue 'outer;
        };
        match settle(&mut commands, batch).await {
            Settled::Quiet(patch) => pending = Some(patch),
            Settled::Closed(patch) => {
                let mut batch = Some(patch);
                let answer = write(&client, &mut batch).await;
                try_report(&mut updates, answer);
                return;
            }
            Settled::Flush(patch, done) => {
                let mut batch = Some(patch);
                let answer = write(&client, &mut batch).await;
                done.send(()).ok();
                report(&mut updates, answer).await;
                pending = batch;
                continue 'outer;
            }
        }
        let answer = write(&client, &mut pending).await;
        report(&mut updates, answer).await;
    }
    let answer = write(&client, &mut pending).await;
    try_report(&mut updates, answer);
}

/// What waiting out the debounce ended with.
enum Settled {
    /// Nothing more arrived; the merged write is ready.
    Quiet(SettingsPatch),
    /// The window asked for a flush while the debounce ran.
    Flush(SettingsPatch, std::sync::mpsc::Sender<()>),
    /// The window's channel closed; write what is pending and stop.
    Closed(SettingsPatch),
}

/// Waits for the debounce while merging later writes; each one restarts the wait.
async fn settle(commands: &mut Receiver<SettingsCommand>, mut pending: SettingsPatch) -> Settled {
    loop {
        tokio::select! {
            () = tokio::time::sleep(debounce_for(&pending)) => return Settled::Quiet(pending),
            command = commands.next() => match command {
                None => return Settled::Closed(pending),
                Some(SettingsCommand::Write(patch)) => pending = merge(Some(pending), *patch),
                Some(SettingsCommand::Flush(done)) => return Settled::Flush(pending, done),
            },
        }
    }
}

/// The wait a write needs: a collapse click is written after its slower debounce.
fn debounce_for(patch: &SettingsPatch) -> Duration {
    let collapse_only = patch.projects.is_some()
        && SettingsPatch {
            projects: None,
            ..patch.clone()
        }
        .is_empty();
    if collapse_only {
        COLLAPSE_DEBOUNCE
    } else {
        SETTINGS_DEBOUNCE
    }
}

/// Folds `next` over `current`; a key both carry keeps the later value.
fn merge(current: Option<SettingsPatch>, next: SettingsPatch) -> SettingsPatch {
    let Some(current) = current else {
        return next;
    };
    match merge_documents(&current, &next) {
        Ok(merged) => merged,
        Err(error) => {
            tracing::warn!(
                target: crate::logging::SERVER,
                %error,
                "[SERVER] Settings write could not be merged"
            );
            next
        }
    }
}

fn merge_documents(
    current: &SettingsPatch,
    next: &SettingsPatch,
) -> Result<SettingsPatch, serde_json::Error> {
    let mut merged = serde_json::to_value(current)?;
    let overlay = serde_json::to_value(next)?;
    if let (serde_json::Value::Object(base), serde_json::Value::Object(overlay)) =
        (&mut merged, overlay)
    {
        base.extend(overlay);
    }
    serde_json::from_value(merged)
}

/// Sends the pending write and returns the answer when it landed; a failure puts the patch
/// back so the next write carries it.
async fn write(client: &Client, pending: &mut Option<SettingsPatch>) -> Option<Settings> {
    let patch = pending.take()?;
    match tokio::time::timeout(WRITE_TIMEOUT, client.update_settings(&patch)).await {
        Ok(Ok(settings)) => Some(settings),
        Ok(Err(error)) => {
            tracing::warn!(
                target: crate::logging::SERVER,
                %error,
                "[SERVER] Settings write failed"
            );
            *pending = Some(patch);
            None
        }
        Err(_) => {
            tracing::warn!(
                target: crate::logging::SERVER,
                "[SERVER] Settings write timed out"
            );
            *pending = Some(patch);
            None
        }
    }
}

/// Reports a settled write; a window that closed meanwhile is not an error.
async fn report(updates: &mut Sender<Update>, answer: Option<Settings>) {
    if let Some(settings) = answer {
        let action = SidebarAction::SettingsLoaded(Box::new(settings));
        sidebar(updates, action).await.ok();
    }
}

/// The close path must not wait on a full update channel: its reader may be gone or busy, so
/// the answer is dropped instead of blocking the writer forever.
fn try_report(updates: &mut Sender<Update>, answer: Option<Settings>) {
    if let Some(settings) = answer {
        let action = SidebarAction::SettingsLoaded(Box::new(settings));
        if updates.try_send(Update::Sidebar(action)).is_err() {
            tracing::debug!(
                target: crate::logging::SERVER,
                "[SERVER] Dropped the settings answer while closing"
            );
        }
    }
}

#[cfg(test)]
#[path = "writer_test.rs"]
mod writer_test;
