//! The CSS line-height arithmetic the picker rows share.
//!
//! The sizes are the semantic typography scale: `--text-meta` is `0.78125rem` and
//! `--text-micro` is `0.75rem` at the 16px root (`design-system.css:40-45`), and every one of
//! those classes sets `line-height: round(1.45em, 1px)` (`typography.css:9-34`).

/// `--text-meta` (`design-system.css:44`): `0.78125rem * 16`.
pub(crate) const META: f32 = 12.5;
/// `--text-micro` (`design-system.css:45`): `0.75rem * 16`.
pub(crate) const MICRO: f32 = 12.;

/// The CSS line-height of a semantic typography size: `round(1.45em, 1px)`
/// (`typography.css:9-34`).
pub(crate) fn line_height(size: f32) -> f32 {
    (size * 1.45).round()
}

#[cfg(test)]
#[path = "text_metrics_test.rs"]
mod text_metrics_test;
