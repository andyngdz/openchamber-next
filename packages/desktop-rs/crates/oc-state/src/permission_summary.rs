//! What a permission request asks for, in the user's terms (`permissionSummary.ts:54-125`).
//!
//! The card turns this into text; the state only decides which facts apply. For directory
//! access the resources are globs such as `/abs/dir/*`, so the summary shows the directory
//! itself and names the file that triggered it.

use std::collections::BTreeMap;

use serde_json::Value;

/// The summary line a permission request gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionSummaryKind {
    /// A directory outside the project.
    ExternalDirectory,
    /// Reading files.
    Read,
    /// Editing, writing or patching files.
    Edit,
    /// Running a command.
    Shell,
    /// A glob search.
    Glob,
    /// A grep search.
    Grep,
    /// Fetching a URL.
    WebFetch,
    /// Searching the web.
    WebSearch,
    /// Loading a skill.
    Skill,
    /// A tool this client cannot explain.
    Tool,
}

/// One path, pattern, query or id the request applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionTarget {
    /// The value shown.
    pub value: String,
    /// Whether `value` is a filesystem path that can be shortened to `~`.
    pub is_path: bool,
    /// A file inside `value` that triggered a directory request.
    pub file: Option<String>,
}

/// The summary of one permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionSummary {
    /// The line to show.
    pub kind: PermissionSummaryKind,
    /// The action name, only for [`PermissionSummaryKind::Tool`].
    pub tool: Option<String>,
    /// What the request applies to.
    pub targets: Vec<PermissionTarget>,
    /// Search scope for glob and grep requests.
    pub scope: Option<String>,
    /// Metadata fully explained by the summary; the raw fallback stays hidden.
    pub metadata_explained: bool,
}

/// The action name the directory summary and the save labels read.
pub(crate) const EXTERNAL_DIRECTORY: &str = "external_directory";
const GLOB: &str = "glob";
const GREP: &str = "grep";
const DIRECTORY_GLOB: &str = "/*";

/// The action names the summary knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionAction {
    ExternalDirectory,
    Read,
    Edit,
    Shell,
    Glob,
    Grep,
    WebFetch,
    WebSearch,
    Skill,
    Tool,
}

fn parse_action(action: &str) -> PermissionAction {
    match action.to_lowercase().as_str() {
        EXTERNAL_DIRECTORY => PermissionAction::ExternalDirectory,
        "read" => PermissionAction::Read,
        "edit" | "write" | "patch" => PermissionAction::Edit,
        "shell" | "bash" | "shell_command" => PermissionAction::Shell,
        GLOB => PermissionAction::Glob,
        GREP => PermissionAction::Grep,
        "webfetch" => PermissionAction::WebFetch,
        "websearch" => PermissionAction::WebSearch,
        "skill" => PermissionAction::Skill,
        _ => PermissionAction::Tool,
    }
}

/// `/abs/dir/*` → `/abs/dir`; anything else is returned unchanged.
pub fn strip_directory_glob(pattern: &str) -> &str {
    if pattern != DIRECTORY_GLOB
        && pattern.ends_with(DIRECTORY_GLOB)
        && pattern.len() > DIRECTORY_GLOB.len()
    {
        &pattern[..pattern.len() - DIRECTORY_GLOB.len()]
    } else {
        pattern
    }
}

fn metadata_text(metadata: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(path)
        .to_owned()
}

fn text_targets(resources: &[String], is_path: bool) -> Vec<PermissionTarget> {
    resources
        .iter()
        .filter(|value| !value.is_empty() && value.as_str() != "*")
        .map(|value| PermissionTarget {
            value: value.clone(),
            is_path,
            file: None,
        })
        .collect()
}

fn explained(kind: PermissionSummaryKind, targets: Vec<PermissionTarget>) -> PermissionSummary {
    PermissionSummary {
        kind,
        tool: None,
        targets,
        scope: None,
        metadata_explained: true,
    }
}

/// The request's summary (`permissionSummary.ts:54-108`).
pub fn summarize_permission(
    action: &str,
    resources: &[String],
    metadata: &BTreeMap<String, Value>,
    has_metadata: bool,
) -> PermissionSummary {
    match parse_action(action) {
        PermissionAction::ExternalDirectory => external_directory_summary(resources, metadata),
        PermissionAction::Read => {
            explained(PermissionSummaryKind::Read, text_targets(resources, true))
        }
        PermissionAction::Edit => {
            explained(PermissionSummaryKind::Edit, text_targets(resources, true))
        }
        PermissionAction::Shell => explained(PermissionSummaryKind::Shell, Vec::new()),
        PermissionAction::Glob => search_summary(PermissionSummaryKind::Glob, resources, metadata),
        PermissionAction::Grep => search_summary(PermissionSummaryKind::Grep, resources, metadata),
        PermissionAction::WebFetch => explained(PermissionSummaryKind::WebFetch, Vec::new()),
        PermissionAction::WebSearch => explained(
            PermissionSummaryKind::WebSearch,
            text_targets(resources, false),
        ),
        PermissionAction::Skill => {
            explained(PermissionSummaryKind::Skill, text_targets(resources, false))
        }
        PermissionAction::Tool => PermissionSummary {
            kind: PermissionSummaryKind::Tool,
            tool: Some(action.to_owned()),
            targets: text_targets(resources, false),
            scope: None,
            metadata_explained: !has_metadata,
        },
    }
}

fn external_directory_summary(
    resources: &[String],
    metadata: &BTreeMap<String, Value>,
) -> PermissionSummary {
    let filepath = metadata_text(metadata, "filepath");
    let parent_dir =
        metadata_text(metadata, "parentDir").map(|dir| strip_directory_glob(&dir).to_owned());
    let targets = resources
        .iter()
        .map(|resource| {
            let value = strip_directory_glob(resource).to_owned();
            let file = filepath.as_deref().and_then(|path| {
                let inside = match &parent_dir {
                    Some(parent) => parent == &value,
                    None => path.starts_with(&format!("{value}/")),
                };
                inside.then(|| file_name(path))
            });
            PermissionTarget {
                value,
                is_path: true,
                file,
            }
        })
        .collect();
    PermissionSummary {
        kind: PermissionSummaryKind::ExternalDirectory,
        tool: None,
        targets,
        scope: None,
        metadata_explained: true,
    }
}

fn search_summary(
    kind: PermissionSummaryKind,
    resources: &[String],
    metadata: &BTreeMap<String, Value>,
) -> PermissionSummary {
    let scope = metadata_text(metadata, "path").filter(|path| path != ".");
    PermissionSummary {
        kind,
        tool: None,
        targets: text_targets(resources, false),
        scope,
        metadata_explained: true,
    }
}

pub(crate) mod save;

pub use save::{can_save_permission, describe_save_patterns};

#[cfg(test)]
#[path = "permission_summary_test.rs"]
mod permission_summary_test;
