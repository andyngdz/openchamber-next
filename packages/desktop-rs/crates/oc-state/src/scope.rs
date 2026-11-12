//! The open session's permission and form scope.
//!
//! Root first, then its descendants breadth-first, each session's requests by `id`
//! (`packages/ui/src/sync/scoped-blocking-requests.ts:5-61`, design "Scope"). Form scopes
//! append the directory's `global` forms after the tree's own (`sync-context.tsx:4089-4095`).

use std::collections::BTreeSet;

use oc_client::form::FormInfo;

use crate::ids::SessionId;
use crate::requests::ScopedPermission;
use crate::state::State;

impl State {
    /// The session ids of `root` and its subtree, root first, breadth-first.
    pub(crate) fn subtree_ids(&self, root: &str) -> Vec<String> {
        let mut order: Vec<String> = Vec::new();
        let mut parent_of: Vec<Option<String>> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for entry in self.sessions.iter().chain(self.global_sessions().iter()) {
            let id = entry.id.as_str().to_owned();
            if !seen.insert(id.clone()) {
                continue;
            }
            order.push(id);
            parent_of.push(entry.parent_id.clone());
        }
        let mut children: std::collections::BTreeMap<&str, Vec<&str>> =
            std::collections::BTreeMap::new();
        for (index, id) in order.iter().enumerate() {
            if let Some(parent) = &parent_of[index]
                && seen.contains(parent)
            {
                children.entry(parent.as_str()).or_default().push(id);
            }
        }
        let mut scoped = Vec::new();
        let mut queue = std::collections::VecDeque::from([root.to_owned()]);
        let mut visited: BTreeSet<String> = BTreeSet::new();
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id.clone()) {
                continue;
            }
            if let Some(next) = children.get(id.as_str()) {
                queue.extend(next.iter().map(|child| (*child).to_owned()));
            }
            scoped.push(id);
        }
        scoped
    }

    /// The open session's permissions, then its descendants' breadth-first
    /// (`scoped-blocking-requests.ts:37-61`).
    pub fn scoped_permissions(&self, open: &SessionId) -> Vec<ScopedPermission> {
        let root = open.as_str();
        let mut shown: BTreeSet<String> = BTreeSet::new();
        let mut result = Vec::new();
        for session in self.subtree_ids(root) {
            for request in self.requests.permissions_for(&session) {
                if !shown.insert(request.id.clone()) {
                    continue;
                }
                result.push(ScopedPermission {
                    request: request.clone(),
                    from_subagent: session != root
                        && self.parent_of(&session).as_deref() == Some(root),
                });
            }
        }
        result
    }

    /// The open session tree's forms, then the directory's `global` forms
    /// (`sync-context.tsx:4089-4095`).
    pub fn scoped_forms(&self, open: &SessionId) -> Vec<FormInfo> {
        let mut forms = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for session in self.subtree_ids(open.as_str()) {
            for form in self.requests.forms_for(&session) {
                if seen.insert(form.id.clone()) {
                    forms.push(form.clone());
                }
            }
        }
        let directory = self
            .open_entry()
            .map(|entry| entry.directory.clone())
            .unwrap_or_else(|| self.directory.clone());
        for form in self.requests.global_forms(&directory) {
            if seen.insert(form.id.clone()) {
                forms.push(form.clone());
            }
        }
        forms
    }

    /// The session's parent, from the listed rows.
    pub(crate) fn parent_of(&self, session: &str) -> Option<String> {
        self.sessions
            .iter()
            .chain(self.global_sessions().iter())
            .find(|entry| entry.id.as_str() == session)
            .and_then(|entry| entry.parent_id.clone())
    }

    /// The directory a listed session lives in.
    pub(crate) fn session_directory(&self, session: &str) -> Option<String> {
        self.sessions
            .iter()
            .chain(self.global_sessions().iter())
            .find(|entry| entry.id.as_str() == session)
            .map(|entry| entry.directory.clone())
    }
}

#[cfg(test)]
#[path = "scope_test.rs"]
mod scope_test;
