//! The sidebar's derived view types: sections, rows and the limits the view reads.
//!
//! Limits: a project and every other row list show 7 rows and 7 more per click; Chats shows 3,
//! then 7 more (`sessionSidebarRowModel.ts:311-316`).

use crate::ids::SessionId;

/// One session row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarRow {
    /// Session id.
    pub id: SessionId,
    /// The row label; `None` until the first turn has been titled.
    pub title: Option<String>,
    /// The busy indicator.
    pub busy: bool,
}

/// One project group's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarProject {
    /// The stored project id.
    pub id: String,
    /// The label the header shows.
    pub label: String,
    /// The absolute project path.
    pub path: String,
    /// `sidebarCollapsed`: the group's rows are hidden until the header is clicked.
    pub collapsed: bool,
}

/// One project group and the rows it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGroup {
    /// The project header.
    pub project: SidebarProject,
    /// Every row the group holds, in sidebar order.
    pub rows: Vec<SidebarRow>,
    /// How many of `rows` the view shows now.
    pub shown: usize,
}

/// One section in the drawn order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarSectionModel {
    /// Managed Chat sessions.
    Chats {
        /// Every Chat row, in sidebar order.
        rows: Vec<SidebarRow>,
        /// How many rows the view shows now.
        shown: usize,
    },
    /// Pins filed in Pinned.
    Pinned {
        /// Every pinned row, in sidebar order.
        rows: Vec<SidebarRow>,
        /// How many rows the view shows now.
        shown: usize,
    },
    /// Sessions whose work state is open.
    InWork {
        /// Every In work row, in sidebar order.
        rows: Vec<SidebarRow>,
        /// How many rows the view shows now.
        shown: usize,
    },
    /// One group per project, in project order.
    Projects {
        /// The project groups.
        groups: Vec<ProjectGroup>,
    },
}

/// The whole sidebar, ready for the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarModel {
    /// The sections, in the saved order.
    pub sections: Vec<SidebarSectionModel>,
    /// Settings, home or the first session page failed; the sidebar shows this with Retry.
    pub error: Option<String>,
    /// Settings, home or the first page is still loading.
    pub loading: bool,
    /// The complete session list has replaced the first page.
    pub complete: bool,
    /// A later page failed; the rows already shown stay.
    pub incomplete: bool,
}

/// Identifies the row list a Show more click extends.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoreKey {
    /// The Chats section.
    Chats,
    /// The Pinned section.
    Pinned,
    /// The In work section.
    Work,
    /// One project's group, by project id.
    Project(String),
}
