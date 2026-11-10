//! Pinned and In work membership, and what each section takes out of its project group.
//!
//! Ported from `SessionProjectCollection.tsx:541-556` (the candidates) and
//! `sections/sectionMembership.ts:90-167` (sections hand works out from the top down).

use std::collections::HashSet;

use super::section::BuiltinSection;
use super::split::Split;
use crate::ids::SessionId;
use crate::session_entry::SessionEntry;
use crate::session_list::SessionList;

/// Whether In work takes its sessions out of their project group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkPlacement {
    /// The section owns its sessions; the project group no longer shows them.
    Claims,
    /// `sidebarKeepWorkInPlace`: the session stays in its project group too.
    StaysInPlace,
}

/// What the built-in sections list, and what leaves a project group.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Membership {
    /// Pinned session ids, in sidebar order.
    pub(crate) pinned: Vec<SessionId>,
    /// In work session ids, in sidebar order.
    pub(crate) work: Vec<SessionId>,
    /// Sessions a section took out of their project group.
    pub(crate) claimed: HashSet<SessionId>,
}

/// Resolves the membership in the order the sections are drawn.
pub(crate) fn resolve(
    sessions: &SessionList,
    split: &Split<'_>,
    sections: &[BuiltinSection],
    placement: WorkPlacement,
) -> Membership {
    let nested = nested_session_ids(sessions);
    let mut membership = Membership::default();
    let mut taken: HashSet<SessionId> = HashSet::new();
    for section in sections {
        match section {
            BuiltinSection::Chats | BuiltinSection::Projects => {}
            BuiltinSection::Pinned => {
                membership.pinned = fresh(pinned_candidates(sessions, split, &nested), &taken);
                for id in &membership.pinned {
                    taken.insert(id.clone());
                    membership.claimed.insert(id.clone());
                }
            }
            BuiltinSection::Work => {
                membership.work = fresh(work_candidates(sessions, split), &taken);
                if placement == WorkPlacement::Claims {
                    for id in &membership.work {
                        taken.insert(id.clone());
                        membership.claimed.insert(id.clone());
                    }
                }
            }
        }
    }
    membership
}

/// A pinned session needs a root of its own (`SessionProjectCollection.tsx:543-549`): a
/// subagent is drawn under its parent, never as a pinned row.
fn pinned_candidates<'a>(
    sessions: &'a SessionList,
    split: &Split<'a>,
    nested: &HashSet<SessionId>,
) -> Vec<&'a SessionEntry> {
    sessions
        .iter()
        .filter(|session| split.is_project_session(&session.id))
        .filter(|session| !nested.contains(&session.id))
        .filter(|session| session.is_pinned_in_default_list())
        .collect()
}

/// A subagent's own work state never puts its run in work (`sidebarWorks.ts:177-181`).
fn work_candidates<'a>(sessions: &'a SessionList, split: &Split<'a>) -> Vec<&'a SessionEntry> {
    sessions
        .iter()
        .filter(|session| split.is_project_session(&session.id))
        .filter(|session| session.parent_id.is_none())
        .filter(|session| session.is_in_work())
        .collect()
}

fn fresh(candidates: Vec<&SessionEntry>, taken: &HashSet<SessionId>) -> Vec<SessionId> {
    candidates
        .into_iter()
        .filter(|session| !taken.contains(&session.id))
        .map(|session| session.id.clone())
        .collect()
}

/// A session nests when the list also holds its parent
/// (`SessionProjectCollection.tsx:312-322`).
fn nested_session_ids(sessions: &SessionList) -> HashSet<SessionId> {
    sessions
        .iter()
        .filter(|session| {
            session
                .parent_id
                .as_deref()
                .is_some_and(|parent| sessions.contains(parent))
        })
        .map(|session| session.id.clone())
        .collect()
}

#[cfg(test)]
#[path = "membership_test.rs"]
mod membership_test;
