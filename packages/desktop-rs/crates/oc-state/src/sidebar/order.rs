//! The project order for every `sidebarProjectSortOrder`.
//!
//! Ported from `sortProjectsByOrder` and `rankByLatestActivity`
//! (`packages/ui/src/components/session/sidebar/list/projectSort.ts:14-74`). `manual` keeps the
//! settings order, because the drag order lives in the Electron renderer's `localStorage`
//! (design Non-Goals).

use oc_client::Project;
use oc_client::settings::SidebarSortOrder;

use super::split::{Split, sort_key};
use crate::session_entry::SessionEntry;

/// The project indices in the order their groups are drawn.
pub(crate) fn order_projects(
    projects: &[Project],
    order: SidebarSortOrder,
    split: &Split<'_>,
) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..projects.len()).collect();
    match order {
        SidebarSortOrder::Manual => {}
        SidebarSortOrder::AZ => indices.sort_by_key(|index| sort_key(&projects[*index])),
        SidebarSortOrder::ZA => {
            indices.sort_by_key(|index| std::cmp::Reverse(sort_key(&projects[*index])));
        }
        SidebarSortOrder::DateAdded => indices.sort_by_key(|index| {
            std::cmp::Reverse(projects[*index].added_at().unwrap_or_default())
        }),
        SidebarSortOrder::Recent => rank_recent(&mut indices, projects, split),
    }
    indices
}

/// The last-opened order, then the newest session activity first; a project with no session
/// keeps its incoming place after the active ones (`projectSort.ts:51-74`).
fn rank_recent(indices: &mut [usize], projects: &[Project], split: &Split<'_>) {
    indices.sort_by_key(|index| {
        std::cmp::Reverse(projects[*index].last_opened_at().unwrap_or_default())
    });
    indices.sort_by_key(|index| std::cmp::Reverse(latest_activity(&split.projects[*index])));
}

fn latest_activity(sessions: &[&SessionEntry]) -> u64 {
    sessions
        .iter()
        .map(|session| session.updated())
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "order_test.rs"]
mod order_test;
