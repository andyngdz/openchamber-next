//! The wire-event reducer, ported from the matching `case`s of `translateWireEvent` in
//! `packages/ui/src/lib/opencode/events.ts` and the sync reducer that applies them.
//!
//! The stream has no directory filter, so an event of a session that is neither listed nor
//! open is dropped. Session-level events (title, status, pending requests) change the sidebar
//! row; message events change the chat column and apply only to the open session.

use oc_client::event::{
    ExecutionInterrupted, MetadataUpdated, SessionCreated, SessionRenamed,
    SessionStatus as StatusEvent, StatusKind,
};
use oc_client::{Event, EventData};

use crate::ids::SessionId;
use crate::message_events::reduce_message_event;
use crate::requests::GLOBAL_SESSION;
use crate::session_entry::SessionEntry;
use crate::session_list::SessionList;
use crate::session_requests::{
    FormSettlement, form_created, form_settled, permission_asked, permission_replied,
    session_deleted,
};
use crate::state::State;
use crate::status::SessionStatus;

/// `session.execution.interrupted` with this reason is OpenCode going away mid-turn: the turn
/// stays open, so the session is not settled (`events.ts`, the `reason === "shutdown"` case).
const SHUTDOWN_REASON: &str = "shutdown";

/// Applies one wire event.
pub(crate) fn reduce_event(state: &mut State, event: &Event) {
    if is_foreign(state, event) {
        return;
    }
    let at = event.created.unwrap_or_default();
    match &event.data {
        EventData::ServerConnected => {}
        EventData::SessionCreated(created) => add_session(state, created, at),
        EventData::SessionRenamed(renamed) => rename_session(state, renamed),
        EventData::SessionDeleted(deleted) => {
            session_deleted(state, &deleted.session_id);
            remove_session(state, &deleted.session_id);
        }
        EventData::SessionMetadataUpdated(updated) => update_metadata(state, updated),
        EventData::ExecutionStarted(started) => {
            set_status(state, &started.session_id, SessionStatus::Busy, at);
        }
        EventData::ExecutionSucceeded(ended) | EventData::SessionIdle(ended) => {
            set_status(state, &ended.session_id, SessionStatus::Idle, at);
        }
        EventData::ExecutionInterrupted(interrupted) => interrupt(state, interrupted, at),
        EventData::ExecutionFailed(failed) => {
            set_status(state, &failed.session_id, SessionStatus::Error, at);
            state.mark_interrupted_turn(&failed.session_id);
        }
        EventData::SessionStatus(status) => apply_status_event(state, event, status, at),
        EventData::PermissionAsked(asked) => permission_asked(state, asked),
        EventData::PermissionReplied(replied) => permission_replied(state, replied),
        EventData::FormCreated(created) => form_created(state, created, &event.location.directory),
        EventData::FormReplied(settled) => {
            form_settled(state, settled, FormSettlement::Replied);
        }
        EventData::FormCancelled(settled) => {
            form_settled(state, settled, FormSettlement::Cancelled);
        }
        EventData::Raw { kind, .. } => drop_unhandled(event, kind),
        _ => reduce_message_event(state, event),
    }
}

/// The event names a session that is neither listed nor open.
fn is_foreign(state: &State, event: &Event) -> bool {
    let Some(session_id) = session_of(&event.data) else {
        return false;
    };
    if state.is_known(session_id) {
        return false;
    }
    tracing::debug!(
        target: crate::logging::STATE,
        session = session_id,
        event = %event.id,
        "[STATE] Dropped event of a session outside every shown group"
    );
    true
}

/// The session an event belongs to; `None` for connection notices, unhandled types and
/// `session.created`, which is admitted by its directory instead.
fn session_of(data: &EventData) -> Option<&str> {
    if let EventData::FormCreated(created) = data
        && created.form.session_id == GLOBAL_SESSION
    {
        return None;
    }
    if let EventData::FormReplied(form) | EventData::FormCancelled(form) = data
        && form.session_id == GLOBAL_SESSION
    {
        return None;
    }
    let id = match data {
        EventData::ServerConnected | EventData::SessionCreated(_) | EventData::Raw { .. } => {
            return None;
        }
        EventData::SessionRenamed(renamed) => &renamed.session_id,
        EventData::SessionMetadataUpdated(updated) => &updated.session_id,
        EventData::SessionDeleted(only)
        | EventData::ExecutionStarted(only)
        | EventData::ExecutionSucceeded(only)
        | EventData::SessionIdle(only) => &only.session_id,
        EventData::InboxEnqueued(enqueued) => &enqueued.session_id,
        EventData::InboxDelivered(delivered) => &delivered.session_id,
        EventData::InboxCancelled(cancelled) => &cancelled.session_id,
        EventData::ExecutionInterrupted(interrupted) => &interrupted.session_id,
        EventData::ExecutionFailed(failed) => &failed.session_id,
        EventData::SessionStatus(status) => &status.session_id,
        EventData::StepStarted(step) => &step.session_id,
        EventData::StepEnded(step) => &step.session_id,
        EventData::TextStarted(text) => &text.session_id,
        EventData::TextDelta(delta) => &delta.session_id,
        EventData::TextEnded(text) => &text.session_id,
        EventData::ToolInputStarted(tool) => &tool.session_id,
        EventData::ToolCalled(tool) => &tool.session_id,
        EventData::ToolSuccess(tool) => &tool.session_id,
        EventData::ToolFailed(tool) => &tool.session_id,
        EventData::PermissionAsked(asked) => &asked.session_id,
        EventData::PermissionReplied(replied) => &replied.session_id,
        EventData::FormCreated(created) => &created.form.session_id,
        EventData::FormReplied(form) | EventData::FormCancelled(form) => &form.session_id,
    };
    Some(id.as_str())
}

fn drop_unhandled(event: &Event, kind: &str) {
    tracing::debug!(
        target: crate::logging::STATE,
        kind,
        event = %event.id,
        "[STATE] Dropped unhandled event"
    );
}

/// `session.created` carries no title; it arrives with `session.renamed`. The session joins
/// the scoped list when its directory is the scoped one, and the shared sidebar when the split
/// below puts its directory in a group (design, State).
fn add_session(state: &mut State, created: &SessionCreated, at: u64) {
    let directory = created.location.directory.as_str();
    let entry = SessionEntry::new(SessionId::from(created.session_id.as_str()), None, at, at)
        .with_directory(directory);
    if directory == state.directory {
        state.sessions.insert(entry.clone());
    }
    if state.sidebar_admits(directory) {
        state.global.list.insert(entry);
    }
    state.reflect_requests();
}

fn rename_session(state: &mut State, renamed: &SessionRenamed) {
    for list in [&mut state.sessions, &mut state.global.list] {
        if let Some(entry) = list.get_mut(&renamed.session_id) {
            entry.title = Some(renamed.title.clone());
        }
    }
}

/// The event carries the whole metadata, so it replaces the work state, the pin and the `/btw`
/// marker, as `events.ts` does for `session.metadata.updated`: metadata without a work state
/// takes the session out of work, and metadata without a pin takes it out of Pinned.
fn update_metadata(state: &mut State, updated: &MetadataUpdated) {
    for list in [&mut state.sessions, &mut state.global.list] {
        if let Some(entry) = list.get_mut(&updated.session_id) {
            entry.work = updated.metadata.work_state();
            entry.pinned = updated.metadata.pinned();
            entry.pin_list = updated.metadata.pin_list().map(str::to_owned);
            entry.btw = updated.metadata.btw();
        }
    }
}

fn remove_session(state: &mut State, session_id: &str) {
    state.sessions.remove(session_id);
    state.global.list.remove(session_id);
    if state.is_open(session_id) {
        state.open = None;
    }
}

fn interrupt(state: &mut State, interrupted: &ExecutionInterrupted, at: u64) {
    if interrupted.reason.as_deref() != Some(SHUTDOWN_REASON) {
        set_status(state, &interrupted.session_id, SessionStatus::Idle, at);
        state.mark_interrupted_turn(&interrupted.session_id);
    }
}

fn apply_status_event(state: &mut State, event: &Event, status: &StatusEvent, at: u64) {
    let next = match status.status {
        StatusKind::Busy | StatusKind::Retry => SessionStatus::Busy,
        StatusKind::Idle => SessionStatus::Idle,
        StatusKind::Other => return drop_unhandled(event, "session.status"),
    };
    set_status(state, &status.session_id, next, at);
}

/// A change between busy and settled moves the session to the top, as in React; a repeat of
/// the same phase does not.
fn set_status(state: &mut State, session_id: &str, status: SessionStatus, at: u64) {
    apply_status(&mut state.sessions, session_id, status, at);
    apply_status(&mut state.global.list, session_id, status, at);
}

fn apply_status(list: &mut SessionList, session_id: &str, status: SessionStatus, at: u64) {
    let Some(entry) = list.get_mut(session_id) else {
        return;
    };
    let was_busy = entry.is_busy();
    entry.record_status(status, at);
    if was_busy != entry.is_busy() {
        list.touch(session_id, at);
    }
}

#[cfg(test)]
#[path = "events_test.rs"]
mod events_test;
