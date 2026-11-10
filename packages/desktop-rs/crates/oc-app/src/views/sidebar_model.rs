//! The sidebar's session groups, as data: which sessions sit In work, which under Projects,
//! and the names each row's controls carry.

use oc_state::{SessionEntry, SessionId, State};

use crate::work::WorkTracker;

const ARCHIVE: &str = "Archive";
const UNTITLED: &str = "Untitled session";

/// The two session groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupKind {
    InWork,
    Projects,
}

impl GroupKind {
    /// The header label, also the group's name in its controls' keys.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::InWork => "In work",
            Self::Projects => "Projects",
        }
    }

    fn row_action(self) -> &'static str {
        match self {
            Self::InWork => "Mark done",
            Self::Projects => "Track as in work",
        }
    }
}

/// One session row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowModel {
    pub(crate) id: SessionId,
    /// The session title; the row's name in its controls' keys.
    pub(crate) title: String,
    /// Drives the busy indicator; not part of any name, since it changes while the row stays.
    pub(crate) busy: bool,
    /// The group action, then Archive.
    pub(crate) actions: [&'static str; 2],
}

/// One group and its rows in sidebar order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupModel {
    pub(crate) kind: GroupKind,
    pub(crate) rows: Vec<RowModel>,
}

/// Groups in sidebar order: In work when it has rows, then Projects.
pub(crate) fn sidebar_groups(state: &State, work: &WorkTracker) -> Vec<GroupModel> {
    let (in_work, rest): (Vec<&SessionEntry>, Vec<&SessionEntry>) = state
        .sessions()
        .iter()
        .partition(|entry| work.is_in_work(&entry.id));
    let group = |kind: GroupKind, entries: Vec<&SessionEntry>| GroupModel {
        kind,
        rows: entries
            .into_iter()
            .map(|entry| row_model(entry, kind))
            .collect(),
    };
    let projects = group(GroupKind::Projects, rest);
    if in_work.is_empty() {
        vec![projects]
    } else {
        vec![group(GroupKind::InWork, in_work), projects]
    }
}

fn row_model(entry: &SessionEntry, kind: GroupKind) -> RowModel {
    let title = entry.title.clone().unwrap_or_else(|| UNTITLED.to_owned());
    RowModel {
        id: entry.id.clone(),
        title,
        busy: entry.is_busy(),
        actions: [kind.row_action(), ARCHIVE],
    }
}

#[cfg(test)]
#[path = "sidebar_model_test.rs"]
mod sidebar_model_test;
