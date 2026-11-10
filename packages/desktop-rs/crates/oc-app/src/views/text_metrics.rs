//! The line-box arithmetic the picker rows and composer triggers share.
//!
//! The sizes are the semantic typography scale: `--text-meta` is `0.78125rem` and
//! `--text-micro` is `0.75rem` at the 16px root (`design-system.css:40-45`), and every one of
//! those classes sets `line-height: round(1.45em, 1px)` (`typography.css:9-34`).
//!
//! The font is Adwaita Sans. Its vertical metrics are the OpenType `hhea`/`OS/2` values in
//! `/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf`: ascent 1984, descent 494, line gap 0,
//! units per em 2048. A specified CSS line-height overrides the font's natural line box, so the
//! strut's descender below the baseline is `(line-height - ascent + descent) / 2`.

/// `--text-meta` (`design-system.css:44`): `0.78125rem * 16`.
pub(crate) const META: f32 = 12.5;
/// `--text-micro` (`design-system.css:45`): `0.75rem * 16`.
pub(crate) const MICRO: f32 = 12.;

const UNITS_PER_EM: f32 = 2048.;
const ASCENT: f32 = 1984.;
const DESCENT: f32 = 494.;
const LINE_GAP: f32 = 0.;

/// The CSS line-height of a semantic typography size: `round(1.45em, 1px)`
/// (`typography.css:9-34`).
pub(crate) fn line_height(size: f32) -> f32 {
    (size * 1.45).round()
}

/// The font's ascent above the baseline at `size`.
pub(crate) fn ascent(size: f32) -> f32 {
    size * ASCENT / UNITS_PER_EM
}

/// The font's descent below the baseline at `size`.
pub(crate) fn descent(size: f32) -> f32 {
    size * DESCENT / UNITS_PER_EM
}

/// The font's line gap at `size`; Adwaita Sans declares none.
pub(crate) fn line_gap(size: f32) -> f32 {
    size * LINE_GAP / UNITS_PER_EM
}

/// How far a line box of the CSS line-height extends below its baseline.
///
/// The content area is the font's ascent plus descent plus line gap; the remaining
/// `line-height - content` is the leading, split evenly above and below. The line gap cancels
/// out, leaving `(line-height - ascent + descent) / 2`.
pub(crate) fn strut_descent(size: f32) -> f32 {
    (line_height(size) - ascent(size) + descent(size) + line_gap(size)) / 2.
}

/// The line box around an ``inline-flex`` label of `inline_height` that has no baseline-
/// participating child: CSS synthesizes its baseline from the box's bottom edge, so the label
/// rises `inline_height` above the baseline and the strut's descender follows below it.
pub(crate) fn inline_flex_line_box(inline_height: f32, size: f32) -> f32 {
    inline_height + strut_descent(size)
}

#[cfg(test)]
#[path = "text_metrics_test.rs"]
mod text_metrics_test;
