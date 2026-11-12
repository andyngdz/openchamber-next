//! The "Always" label and the patterns it saves (`permissionSummary.ts:116-125`).

use super::{EXTERNAL_DIRECTORY, strip_directory_glob};

/// Whether an "always" reply would save anything.
pub fn can_save_permission(save: Option<&[String]>) -> bool {
    save.is_some_and(|patterns| !patterns.is_empty())
}

/// Patterns an "always" reply saves, as the user reads them.
pub fn describe_save_patterns(action: &str, save: &[String]) -> Vec<String> {
    if save.is_empty() || save.iter().all(|pattern| pattern == "*") {
        return Vec::new();
    }
    if action.eq_ignore_ascii_case(EXTERNAL_DIRECTORY) {
        save.iter()
            .map(|pattern| strip_directory_glob(pattern).to_owned())
            .collect()
    } else {
        save.to_vec()
    }
}

#[cfg(test)]
#[path = "save_test.rs"]
mod save_test;
