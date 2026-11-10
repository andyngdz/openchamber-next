//! Builds the derived sidebar sections from settings, home info and the global list.

use std::collections::BTreeMap;

use oc_client::filesystem::Home;
use oc_client::settings::Settings;

use super::membership::{self, Membership, WorkPlacement};
use super::model::{MoreKey, ProjectGroup, SidebarProject, SidebarRow, SidebarSectionModel};
use super::order::order_projects;
use super::section::{self, BuiltinSection};
use super::split::{ChatRoots, Split, display_label, split};
use crate::ids::SessionId;
use crate::session_entry::SessionEntry;
use crate::session_list::SessionList;

/// Chats opens on a short page (`sessionSidebarRowModel.ts:312`).
const CHATS_INITIAL_LIMIT: usize = 3;
/// One page of sessions (`sessionSidebarRowModel.ts:314-316`).
const SESSIONS_INITIAL_LIMIT: usize = 7;
/// One click of Show more.
const PAGE_SIZE: usize = 7;

/// Everything a section needs to draw, in one borrow so section builders stay small.
struct Context<'a> {
    settings: &'a Settings,
    home: &'a Home,
    sessions: &'a SessionList,
    split: &'a Split<'a>,
    project_order: &'a [usize],
    membership: &'a Membership,
    more: &'a BTreeMap<MoreKey, u32>,
}

/// Builds the sections from the shared settings, the home info and the global session list.
pub(crate) fn sections(
    settings: &Settings,
    home: &Home,
    sessions: &SessionList,
    more: &BTreeMap<MoreKey, u32>,
) -> Vec<SidebarSectionModel> {
    let Some(roots) = ChatRoots::from_home(home) else {
        return Vec::new();
    };
    let split = split(sessions, &settings.projects, &roots);
    let project_order = order_projects(
        &settings.projects,
        settings.sidebar_project_sort_order,
        &split,
    );
    let visible = section::visible(settings);
    let membership = membership::resolve(sessions, &split, &visible, work_placement(settings));
    let context = Context {
        settings,
        home,
        sessions,
        split: &split,
        project_order: &project_order,
        membership: &membership,
        more,
    };
    visible
        .into_iter()
        .map(|section| build_section(section, &context))
        .collect()
}

fn work_placement(settings: &Settings) -> WorkPlacement {
    if settings.sidebar_keep_work_in_place {
        WorkPlacement::StaysInPlace
    } else {
        WorkPlacement::Claims
    }
}

fn build_section(section: BuiltinSection, context: &Context<'_>) -> SidebarSectionModel {
    match section {
        BuiltinSection::Chats => {
            let rows = rows_of(context.split.chats.iter().copied());
            let shown = shown(&rows, MoreKey::Chats, CHATS_INITIAL_LIMIT, context.more);
            SidebarSectionModel::Chats { rows, shown }
        }
        BuiltinSection::Pinned => {
            let rows = rows_of_ids(&context.membership.pinned, context.sessions);
            let shown = shown(&rows, MoreKey::Pinned, SESSIONS_INITIAL_LIMIT, context.more);
            SidebarSectionModel::Pinned { rows, shown }
        }
        BuiltinSection::Work => {
            let rows = rows_of_ids(&context.membership.work, context.sessions);
            let shown = shown(&rows, MoreKey::Work, SESSIONS_INITIAL_LIMIT, context.more);
            SidebarSectionModel::InWork { rows, shown }
        }
        BuiltinSection::Projects => SidebarSectionModel::Projects {
            groups: context
                .project_order
                .iter()
                .map(|index| project_group(*index, context))
                .collect(),
        },
    }
}

fn project_group(index: usize, context: &Context<'_>) -> ProjectGroup {
    let project = &context.settings.projects[index];
    let id = project.id().unwrap_or_default().to_owned();
    let rows = rows_of(
        context.split.projects[index]
            .iter()
            .copied()
            .filter(|session| !context.membership.claimed.contains(&session.id)),
    );
    let shown = shown(
        &rows,
        MoreKey::Project(id.clone()),
        SESSIONS_INITIAL_LIMIT,
        context.more,
    );
    ProjectGroup {
        project: SidebarProject {
            id,
            label: display_label(project, &context.home.home),
            path: project.path().unwrap_or_default().to_owned(),
            collapsed: project.sidebar_collapsed().unwrap_or(false),
        },
        rows,
        shown,
    }
}

fn rows_of<'a>(entries: impl Iterator<Item = &'a SessionEntry>) -> Vec<SidebarRow> {
    entries.map(row).collect()
}

fn rows_of_ids(ids: &[SessionId], sessions: &SessionList) -> Vec<SidebarRow> {
    ids.iter()
        .filter_map(|id| sessions.get(id))
        .map(row)
        .collect()
}

fn row(entry: &SessionEntry) -> SidebarRow {
    SidebarRow {
        id: entry.id.clone(),
        title: entry.title.clone(),
        busy: entry.is_busy(),
    }
}

fn shown(
    rows: &[SidebarRow],
    key: MoreKey,
    initial: usize,
    more: &BTreeMap<MoreKey, u32>,
) -> usize {
    let pages = more.get(&key).copied().unwrap_or_default() as usize;
    rows.len().min(initial + PAGE_SIZE * pages)
}

#[cfg(test)]
#[path = "build_test.rs"]
mod build_test;
