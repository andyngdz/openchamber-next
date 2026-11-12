//! The permission and form events, applied to the requests and recovery stores.

use oc_client::event::{FormCreated, FormReplied, PermissionAsked, PermissionReplied};

use crate::state::State;

/// How a form's settle event reads for recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormSettlement {
    /// `form.replied`: the user answered it.
    Replied,
    /// `form.cancelled`: turn death may have cancelled the form, so it is not conclusive.
    Cancelled,
}

/// A fresh request, held back when the server may answer it.
pub(crate) fn permission_asked(state: &mut State, asked: &PermissionAsked) {
    state.add_fresh_permission(asked.request.clone());
}

/// An answered request leaves the store and its holds.
pub(crate) fn permission_replied(state: &mut State, replied: &PermissionReplied) {
    state.settle_permission(&replied.session_id, &replied.request_id);
}

/// A new form joins the store and the recovery list, under the event's directory.
pub(crate) fn form_created(state: &mut State, created: &FormCreated, event_directory: &str) {
    let form = &created.form;
    let directory = if event_directory.is_empty() {
        state
            .session_directory(&form.session_id)
            .unwrap_or_else(|| state.directory.clone())
    } else {
        event_directory.to_owned()
    };
    state.recoverable.note_form(form, &directory);
    state.requests.upsert_form(form.clone(), &directory);
    state.reflect_requests();
}

/// A settled form leaves the store; only a reply is conclusive for recovery.
pub(crate) fn form_settled(state: &mut State, settled: &FormReplied, settlement: FormSettlement) {
    state.requests.remove_form(&settled.session_id, &settled.id);
    if settlement == FormSettlement::Replied {
        state.recoverable.note_replied(&settled.id);
    }
    state.reflect_requests();
}

/// A deleted session takes its requests and questions with it.
pub(crate) fn session_deleted(state: &mut State, session: &str) {
    state.requests.drop_session(session);
    state.recoverable.drop_session(session);
    state.reflect_requests();
}

#[cfg(test)]
#[path = "session_requests_test.rs"]
mod session_requests_test;
