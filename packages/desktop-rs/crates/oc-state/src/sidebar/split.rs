//! Splits the global session list into Chats and one bucket per project.
//!
//! Ported from `partitionSidebarSessions` and `isKnownActiveSessionDirectory`
//! (`packages/ui/src/components/session/sidebar/list/sessionCollection.ts:41-75`) and
//! `splitGlobalSessionsByArchived` (`packages/ui/src/stores/globalSessions.ts:29-39`);
//! the four chat roots are `packages/ui/src/lib/chatDirectories.ts:16-49,70-79`.

use std::collections::HashMap;

use oc_client::Project;
use oc_client::filesystem::Home;

use crate::ids::SessionId;
use crate::session_entry::SessionEntry;
use crate::session_list::SessionList;

/// The four roots a Chat session lives under (`chatDirectories.ts:70-79`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatRoots {
    configured: String,
    canonical_configured: String,
    legacy: String,
    canonical_legacy: String,
}

impl ChatRoots {
    /// The roots the server named, each falling back as `getChatRoots` does.
    pub(crate) fn from_home(home: &Home) -> Option<Self> {
        let home_path = normalize_path(&home.home)?;
        let legacy = join_path(&home_path, ".config/openchamber/chats");
        let configured = normalize_path(&home.chats_root).unwrap_or_else(|| legacy.clone());
        let canonical_configured = home
            .canonical_chats_root
            .as_deref()
            .and_then(normalize_path)
            .unwrap_or_else(|| configured.clone());
        let canonical_legacy = home
            .canonical_legacy_chats_root
            .as_deref()
            .and_then(normalize_path)
            .unwrap_or_else(|| legacy.clone());
        Some(Self {
            configured,
            canonical_configured,
            legacy,
            canonical_legacy,
        })
    }

    /// Whether the directory lies under one of the four roots (`chatDirectories.ts:30-42`).
    pub(crate) fn contains(&self, directory: &str) -> bool {
        let Some(directory) = normalize_path(directory) else {
            return false;
        };
        [
            &self.configured,
            &self.canonical_configured,
            &self.legacy,
            &self.canonical_legacy,
        ]
        .into_iter()
        .any(|root| is_within_root(&directory, root))
    }
}

/// The visible sessions, split; archived and `/btw` sessions are already gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Split<'a> {
    /// Chat sessions, in sidebar order.
    pub(crate) chats: Vec<&'a SessionEntry>,
    /// One bucket per project, parallel to the projects slice, in sidebar order.
    pub(crate) projects: Vec<Vec<&'a SessionEntry>>,
    /// The project index of every session that belongs to a project.
    pub(crate) project_of: HashMap<SessionId, usize>,
}

impl Split<'_> {
    /// The session belongs to a project, not Chats.
    pub(crate) fn is_project_session(&self, id: &SessionId) -> bool {
        self.project_of.contains_key(id)
    }
}

/// Splits the list on the four chat roots and the project paths.
pub(crate) fn split<'a>(
    sessions: &'a SessionList,
    projects: &[Project],
    roots: &ChatRoots,
) -> Split<'a> {
    let mut split = Split {
        chats: Vec::new(),
        projects: vec![Vec::new(); projects.len()],
        project_of: HashMap::new(),
    };
    for session in sessions.iter() {
        if session.archived || session.btw {
            continue;
        }
        if roots.contains(&session.directory) {
            split.chats.push(session);
        } else if let Some(index) = project_index(&session.directory, projects) {
            split.projects[index].push(session);
            split.project_of.insert(session.id.clone(), index);
        }
    }
    split
}

/// The project whose path most closely contains `directory`; nested projects share a prefix,
/// and the closest one owns the session, as `resolveOwner` walks up the tree
/// (`packages/ui/src/components/session/sidebar/sessions/sessionOwnership.ts:195-211`).
pub(crate) fn project_index(directory: &str, projects: &[Project]) -> Option<usize> {
    let directory = normalize_path(directory)?;
    let mut best: Option<(usize, usize)> = None;
    for (index, project) in projects.iter().enumerate() {
        let Some(raw_path) = project.path() else {
            continue;
        };
        let Some(path) = normalize_path(raw_path) else {
            continue;
        };
        if !is_under(&directory, &path) {
            continue;
        }
        let length = path.len();
        if best.is_none_or(|(_, best_length)| length > best_length) {
            best = Some((index, length));
        }
    }
    best.map(|(index, _)| index)
}

/// The label the sidebar shows: the stored label, else the folder name (`useDraftTarget.ts:73`).
pub(crate) fn display_label(project: &Project, home: &str) -> String {
    let label = project.label().map(str::trim).unwrap_or_default();
    if !label.is_empty() {
        return label.to_owned();
    }
    format_directory_name(project.path().unwrap_or_default(), home)
}

/// The key `compareLabels` sorts on: the label, else the whole path (`projectSort.ts:14-15`).
pub(crate) fn sort_key(project: &Project) -> String {
    project
        .label()
        .filter(|label| !label.is_empty())
        .or_else(|| project.path())
        .unwrap_or_default()
        .to_lowercase()
}

/// The last path segment, or `~` for the home directory (`formatDirectoryName`,
/// `packages/ui/src/lib/utils.ts:99-116`).
pub(crate) fn format_directory_name(path: &str, home: &str) -> String {
    let Some(normalized) = normalize_path(path) else {
        return "/".to_owned();
    };
    if normalize_path(home).as_deref() == Some(normalized.as_str()) && normalized != "/" {
        return "~".to_owned();
    }
    normalized
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .map_or_else(|| "/".to_owned(), str::to_owned)
}

/// `normalizePath`: backslashes to slashes, a lowercase Windows drive uppercased, trailing
/// slashes trimmed (`packages/ui/src/lib/pathNormalization.ts:21-36`).
pub(crate) fn normalize_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let replaced = uppercase_drive(&trimmed.replace('\\', "/"));
    if replaced == "/" {
        return Some(replaced);
    }
    let stripped = replaced.trim_end_matches('/');
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_owned())
    }
}

fn uppercase_drive(path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_lowercase() {
        let mut out = String::with_capacity(path.len());
        out.push(bytes[0].to_ascii_uppercase() as char);
        out.push_str(&path[1..]);
        return out;
    }
    path.to_owned()
}

fn join_path(base: &str, rest: &str) -> String {
    format!("{}/{rest}", base.trim_end_matches('/'))
}

/// `isNormalizedPathWithinProject` (`packages/ui/src/components/session/sidebar/utils.tsx:114-119`).
fn is_under(directory: &str, root: &str) -> bool {
    directory == root
        || directory
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// `isWithinRoot`: the directory must not carry `.` or `..` segments (`chatDirectories.ts:21-24`).
fn is_within_root(directory: &str, root: &str) -> bool {
    !directory.split('/').any(|part| part == ".." || part == ".") && is_under(directory, root)
}

#[cfg(test)]
#[path = "split_test.rs"]
mod split_test;
