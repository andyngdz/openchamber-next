//! Patterns a permission card still needs to list (`permissionCardPatterns.ts:6-10`).
//!
//! A v2 shell request carries no metadata, so the card renders its requested commands as
//! the command block; any pattern already visible there would only repeat itself.

/// Drops the patterns the rendered command already shows.
pub fn get_visible_permission_patterns(patterns: &[String], rendered_command: &str) -> Vec<String> {
    if rendered_command.is_empty() {
        return patterns.to_vec();
    }
    let rendered: std::collections::BTreeSet<&str> = rendered_command.split('\n').collect();
    patterns
        .iter()
        .filter(|pattern| !rendered.contains(pattern.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "permission_patterns_test.rs"]
mod permission_patterns_test;
