//! One composer send, as the window hands it to the I/O task (`oc_state::SendPlan`; design
//! "Send decision").
//!
//! The window resolves the plan's dismiss targets to the calls, directories and records the
//! task needs; the task runs the dismissals in parallel, then the action the plan chose.

use oc_client::event::{FormCreated, FormReplied, PermissionAsked, PermissionReplied};
use oc_client::form::FormInfo;
use oc_client::permission::PermissionRequest;
use oc_client::types::{Location, ModelRef};
use oc_client::{Event, EventData};
use oc_state::{DismissKind, DismissTarget, SendPlan, SessionId, State};

/// One pending request a send dismisses, with the record it restores when the call fails.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DismissCall {
    /// The directory the request lives in.
    pub(crate) directory: String,
    /// The session the request belongs to.
    pub(crate) session_id: String,
    /// The request or form id.
    pub(crate) id: String,
    /// Which call answers it.
    pub(crate) kind: DismissKind,
    /// The permission record, when this dismisses a permission.
    pub(crate) permission: Option<PermissionRequest>,
    /// The form record, when this dismisses a form.
    pub(crate) form: Option<FormInfo>,
}

/// What a send does after its dismissal step.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SendAction {
    /// Queue the text and dismiss nothing.
    Enqueue {
        /// The text to queue.
        text: String,
    },
    /// The dismissal step removed something and the send is not a steer: stop there and
    /// queue the composer text, if any.
    DismissThenEnqueue {
        /// The composer text, absent when the composer was empty.
        text: Option<String>,
    },
    /// Take the queue when it holds items, then send the joined text.
    DismissThenSend {
        /// The session's queue holds items, so take the whole queue first.
        take_queue: bool,
        /// The composer text that follows the taken queue in the join.
        text: String,
        /// Steer the text into the running turn instead of sending a plain prompt.
        steer: bool,
    },
}

/// One composer send, with everything the I/O task needs to run it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SendExecution {
    /// The open session.
    pub(crate) session: SessionId,
    /// The directory it lives in.
    pub(crate) directory: String,
    /// The agent the composer picked.
    pub(crate) agent: Option<String>,
    /// The model the composer picked.
    pub(crate) model: Option<ModelRef>,
    /// The model the state's session record holds, which the model compare reads.
    pub(crate) record_model: Option<ModelRef>,
    /// The pending requests the send answers first.
    pub(crate) dismiss: Vec<DismissCall>,
    /// What the send does after the dismissals.
    pub(crate) action: SendAction,
}

/// Turns `plan` into the calls the I/O task runs, resolving each dismiss target's directory
/// and the record it restores. `None` when the plan sends nothing.
pub(crate) fn from_plan(
    plan: &SendPlan,
    state: &State,
    agent: Option<String>,
    model: Option<ModelRef>,
) -> Option<SendExecution> {
    let open = state.open()?;
    let directory = state
        .open_entry()
        .map(|entry| entry.directory.clone())
        .unwrap_or_else(|| state.directory().to_owned());
    let record_model = state
        .session_record(&open.id)
        .and_then(|record| record.model.clone());
    let (targets, action) = match plan {
        SendPlan::Nothing => return None,
        SendPlan::Enqueue { text } => (Vec::new(), SendAction::Enqueue { text: text.clone() }),
        SendPlan::DismissThenEnqueue { dismiss, text } => (
            dismiss.clone(),
            SendAction::DismissThenEnqueue { text: text.clone() },
        ),
        SendPlan::DismissThenSend { dismiss, prompt } => (
            dismiss.clone(),
            SendAction::DismissThenSend {
                take_queue: prompt.take_queue,
                text: prompt.composer_text.clone(),
                steer: prompt.steer,
            },
        ),
    };
    let root = open.id.as_str();
    let dismiss = targets
        .iter()
        .filter_map(|target| dismiss_call(state, root, target))
        .collect();
    Some(SendExecution {
        session: open.id.clone(),
        directory,
        agent,
        model,
        record_model,
        dismiss,
        action,
    })
}

/// One target as the call it needs, or `None` when the request left the store already.
fn dismiss_call(state: &State, root: &str, target: &DismissTarget) -> Option<DismissCall> {
    let directory = session_directory(state, &target.session_id);
    match target.kind {
        DismissKind::Permission => state
            .scoped_permissions(&SessionId::from(root))
            .iter()
            .find(|shown| shown.request.id == target.id)
            .map(|shown| DismissCall {
                directory,
                session_id: target.session_id.clone(),
                id: target.id.clone(),
                kind: target.kind,
                permission: Some(shown.request.clone()),
                form: None,
            }),
        DismissKind::Form => state
            .scoped_forms(&SessionId::from(root))
            .into_iter()
            .find(|form| form.id == target.id)
            .map(|form| DismissCall {
                directory,
                session_id: target.session_id.clone(),
                id: target.id.clone(),
                kind: target.kind,
                permission: None,
                form: Some(form),
            }),
    }
}

/// The directory a session lives in, from either list, else the app's own.
fn session_directory(state: &State, session: &str) -> String {
    state
        .sessions()
        .iter()
        .chain(state.global_sessions().iter())
        .find(|entry| entry.id.as_str() == session)
        .map(|entry| entry.directory.clone())
        .filter(|directory| !directory.is_empty())
        .unwrap_or_else(|| state.directory().to_owned())
}

/// The event that takes a request off screen before its call answers.
pub(crate) fn settled_event(call: &DismissCall) -> Option<Event> {
    let event_data = match (&call.permission, &call.form) {
        (Some(permission), _) => EventData::PermissionReplied(PermissionReplied {
            request_id: permission.id.clone(),
            session_id: permission.session_id.clone(),
        }),
        (None, Some(form)) => EventData::FormCancelled(FormReplied {
            id: form.id.clone(),
            session_id: form.session_id.clone(),
        }),
        (None, None) => return None,
    };
    Some(local_event(&call.directory, event_data))
}

/// The event that puts a request back after its call failed.
pub(crate) fn restored_event(call: &DismissCall) -> Option<Event> {
    let event_data = match (&call.permission, &call.form) {
        (Some(permission), _) => EventData::PermissionAsked(PermissionAsked {
            id: permission.id.clone(),
            session_id: permission.session_id.clone(),
            request: permission.clone(),
        }),
        (None, Some(form)) => EventData::FormCreated(FormCreated { form: form.clone() }),
        (None, None) => return None,
    };
    Some(local_event(&call.directory, event_data))
}

/// An event the app applies as the server would send it; the id is not a server id.
fn local_event(directory: &str, event_data: EventData) -> Event {
    Event {
        id: String::new(),
        created: None,
        location: Location {
            directory: directory.to_owned(),
        },
        data: event_data,
    }
}

#[cfg(test)]
#[path = "execution_test.rs"]
mod execution_test;
