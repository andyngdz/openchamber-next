//! Pending permission requests and forms, by session, as the server reports them.
//!
//! `permission.asked` and `form.created` upsert, their settled events remove, and a list
//! result replaces exactly one directory's entries. A failed list keeps every entry it had
//! and marks the directory stale, so an empty dock is never shown as fact
//! (`packages/ui/src/sync/event-reducer.ts:697-751`, design "Pending requests").

use std::collections::{BTreeMap, BTreeSet};

use oc_client::form::FormInfo;
use oc_client::permission::PermissionRequest;

use crate::ids::RequestId;
use crate::session_entry::PendingRequest;

/// The `sessionID` a location-scoped form carries.
pub(crate) const GLOBAL_SESSION: &str = "global";
/// The form metadata key that names what kind of form it is.
pub(crate) const METADATA_KIND: &str = "kind";

/// Pending requests of every session, each session's list sorted by `id`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Requests {
    permissions: BTreeMap<String, Vec<PermissionRequest>>,
    forms: BTreeMap<String, Vec<FormInfo>>,
    /// The directory each session's permissions were loaded from, so a replace can clear
    /// exactly that directory's entries.
    permission_dirs: BTreeMap<String, String>,
    /// The directory each session's forms were loaded from.
    form_dirs: BTreeMap<String, String>,
    /// The directory's `global` forms.
    global_forms: BTreeMap<String, Vec<FormInfo>>,
    /// Directories whose last permission list failed; their entries stay as they were.
    stale_permissions: BTreeSet<String>,
    /// Directories whose last form list failed; their entries stay as they were.
    stale_forms: BTreeSet<String>,
}

impl Requests {
    /// The session's pending permissions, by `id`.
    pub(crate) fn permissions_for(&self, session: &str) -> &[PermissionRequest] {
        self.permissions.get(session).map_or(&[], Vec::as_slice)
    }

    /// The session's pending forms, by `id`.
    pub(crate) fn forms_for(&self, session: &str) -> &[FormInfo] {
        self.forms.get(session).map_or(&[], Vec::as_slice)
    }

    /// The directory's location-scoped forms, by `id`.
    pub(crate) fn global_forms(&self, directory: &str) -> &[FormInfo] {
        self.global_forms.get(directory).map_or(&[], Vec::as_slice)
    }

    /// Adds or replaces one request (`event-reducer.ts:700-707`).
    pub(crate) fn upsert_permission(&mut self, request: PermissionRequest, directory: &str) {
        let session = request.session_id.clone();
        let list = self.permissions.entry(session.clone()).or_default();
        match list.binary_search_by(|entry| entry.id.as_str().cmp(request.id.as_str())) {
            Ok(index) => list[index] = request,
            Err(index) => list.insert(index, request),
        }
        self.permission_dirs.insert(session, directory.to_owned());
    }

    /// Adds or replaces one form; a `global` form is kept under its directory.
    pub(crate) fn upsert_form(&mut self, form: FormInfo, directory: &str) {
        if form.session_id == GLOBAL_SESSION {
            let list = self.global_forms.entry(directory.to_owned()).or_default();
            match list.binary_search_by(|entry| entry.id.as_str().cmp(form.id.as_str())) {
                Ok(index) => list[index] = form,
                Err(index) => list.insert(index, form),
            }
        } else {
            let session = form.session_id.clone();
            let list = self.forms.entry(session.clone()).or_default();
            match list.binary_search_by(|entry| entry.id.as_str().cmp(form.id.as_str())) {
                Ok(index) => list[index] = form,
                Err(index) => list.insert(index, form),
            }
            self.form_dirs.insert(session, directory.to_owned());
        }
    }

    /// Drops one answered or rejected request.
    pub(crate) fn remove_permission(&mut self, session: &str, id: &str) {
        let Some(list) = self.permissions.get_mut(session) else {
            return;
        };
        list.retain(|entry| entry.id != id);
        if list.is_empty() {
            self.permissions.remove(session);
            self.permission_dirs.remove(session);
        }
    }

    /// Drops one settled form, wherever its directory held it.
    pub(crate) fn remove_form(&mut self, session: &str, id: &str) {
        if session == GLOBAL_SESSION {
            for list in self.global_forms.values_mut() {
                list.retain(|entry| entry.id != id);
            }
            return;
        }
        let Some(list) = self.forms.get_mut(session) else {
            return;
        };
        list.retain(|entry| entry.id != id);
        if list.is_empty() {
            self.forms.remove(session);
            self.form_dirs.remove(session);
        }
    }

    /// Replaces exactly `directory`'s requests; a failure keeps this whole list.
    pub(crate) fn replace_permissions(
        &mut self,
        directory: &str,
        requests: Vec<PermissionRequest>,
    ) {
        let replaced: Vec<String> = self
            .permission_dirs
            .iter()
            .filter(|(_, held)| held.as_str() == directory)
            .map(|(session, _)| session.clone())
            .collect();
        for session in replaced {
            self.permissions.remove(&session);
            self.permission_dirs.remove(&session);
        }
        for request in requests {
            self.upsert_permission(request, directory);
        }
        self.stale_permissions.remove(directory);
    }

    /// Replaces exactly `directory`'s forms, the `global` ones included.
    pub(crate) fn replace_forms(&mut self, directory: &str, forms: Vec<FormInfo>) {
        let replaced: Vec<String> = self
            .form_dirs
            .iter()
            .filter(|(_, held)| held.as_str() == directory)
            .map(|(session, _)| session.clone())
            .collect();
        for session in replaced {
            self.forms.remove(&session);
            self.form_dirs.remove(&session);
        }
        self.global_forms.remove(directory);
        for form in forms {
            self.upsert_form(form, directory);
        }
        self.stale_forms.remove(directory);
    }

    /// A failed permission list keeps the entries and marks the directory stale.
    pub(crate) fn fail_permissions(&mut self, directory: &str) {
        self.stale_permissions.insert(directory.to_owned());
    }

    /// A failed form list keeps the entries and marks the directory stale.
    pub(crate) fn fail_forms(&mut self, directory: &str) {
        self.stale_forms.insert(directory.to_owned());
    }

    /// Drops every request and form of a deleted session.
    pub(crate) fn drop_session(&mut self, session: &str) {
        self.permissions.remove(session);
        self.permission_dirs.remove(session);
        self.forms.remove(session);
        self.form_dirs.remove(session);
    }

    /// Whether the directory's last permission or form list failed.
    pub(crate) fn is_stale(&self, directory: &str) -> bool {
        self.stale_permissions.contains(directory) || self.stale_forms.contains(directory)
    }

    /// Whether the request is on screen.
    pub(crate) fn has_permission(&self, session: &str, id: &str) -> bool {
        self.permissions_for(session)
            .iter()
            .any(|entry| entry.id == id)
    }

    /// Whether the form is on screen.
    #[cfg(test)]
    pub(crate) fn has_form(&self, id: &str) -> bool {
        self.forms
            .values()
            .any(|list| list.iter().any(|entry| entry.id == id))
            || self
                .global_forms
                .values()
                .any(|list| list.iter().any(|entry| entry.id == id))
    }

    /// The sidebar row's pending marks for one session.
    pub(crate) fn pending_for(&self, session: &str) -> Vec<PendingRequest> {
        let mut pending: Vec<PendingRequest> = self
            .permissions_for(session)
            .iter()
            .map(|request| PendingRequest::Permission(RequestId::from(request.id.as_str())))
            .collect();
        pending.extend(
            self.forms_for(session)
                .iter()
                .map(|form| PendingRequest::Form(RequestId::from(form.id.as_str()))),
        );
        pending
    }
}

/// A request shown in the dock, with React's subagent mark (`usePermissionResponse.ts:40-48`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScopedPermission {
    /// The request itself.
    pub request: PermissionRequest,
    /// Its session is a direct child of the open session.
    pub from_subagent: bool,
}

#[cfg(test)]
#[path = "requests_test.rs"]
mod requests_test;
