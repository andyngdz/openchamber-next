//! The line-box arithmetic the picker rows and composer triggers share.
//!
//! The sizes are the semantic typography scale: `--text-meta` is `0.78125rem` and
//! `--text-micro` is `0.75rem` at the 16px root (`design-system.css:40-45`), and every one of
//! those classes sets `line-height: round(1.45em, 1px)` (`typography.css:9-34`).
//!
//! A specified CSS line-height overrides the font's natural line box, so the strut's descender
//! below the baseline is `(line-height - ascent + descent) / 2`. The ascent and descent are read
//! from GPUI's text system for the font the UI family resolves to at render time, the font GPUI
//! actually shapes with. The capture's font is Adwaita Sans (ascent 1984, descent 494 at 2048
//! units per em), so its rows keep the reference values; a machine that resolves another font
//! gets that font's line boxes.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, TextSystem, px};

/// `--text-meta` (`design-system.css:44`): `0.78125rem * 16`.
pub(crate) const META: f32 = 12.5;
/// `--text-micro` (`design-system.css:45`): `0.75rem * 16`.
pub(crate) const MICRO: f32 = 12.;

/// The family gpui-component names until it resolves the real UI font on the theme.
#[cfg(test)]
const SYSTEM_UI_FAMILY: &str = ".SystemUIFont";

/// The CSS line-height of a semantic typography size: `round(1.45em, 1px)`
/// (`typography.css:9-34`).
pub(crate) fn line_height(size: f32) -> f32 {
    (size * 1.45).round()
}

/// The resolved UI font's ascent and descent at one size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UiMetrics {
    /// Pixels above the baseline at the size the metrics were read for.
    pub(crate) ascent: f32,
    /// Pixels below the baseline at that size.
    pub(crate) descent: f32,
}

impl UiMetrics {
    /// The metrics of `family`'s resolved font at `size`, as GPUI shapes it.
    pub(crate) fn read(text_system: &TextSystem, family: &str, size: f32) -> Self {
        let font_id = text_system.resolve_font(&gpui_kit::font(family.to_owned()));
        Self {
            ascent: f32::from(text_system.ascent(font_id, px(size))),
            // GPUI's backends report the descent with either sign; the distance below the
            // baseline is its magnitude.
            descent: f32::from(text_system.descent(font_id, px(size))).abs(),
        }
    }

    /// The metrics of the app's current UI family at `size`.
    pub(crate) fn ui(cx: &App, size: f32) -> Self {
        Self::read(cx.text_system(), cx.theme().font_family.as_ref(), size)
    }
}

/// How far a line box of the CSS line-height extends below its baseline.
///
/// The content area is the font's ascent plus descent; the remaining
/// `line-height - content` is the leading, split evenly above and below, leaving
/// `(line-height - ascent + descent) / 2`.
pub(crate) fn strut_descent(metrics: UiMetrics, size: f32) -> f32 {
    (line_height(size) - metrics.ascent + metrics.descent) / 2.
}

/// The line box around an ``inline-flex`` label of `inline_height` that has no baseline-
/// participating child: CSS synthesizes its baseline from the box's bottom edge, so the label
/// rises `inline_height` above the baseline and the strut's descender follows below it.
pub(crate) fn inline_flex_line_box(metrics: UiMetrics, inline_height: f32, size: f32) -> f32 {
    inline_height + strut_descent(metrics, size)
}

/// The UI font a test environment resolves, with its metrics at `size`.
#[cfg(test)]
pub(crate) fn resolved_ui_metrics(size: f32) -> (String, UiMetrics) {
    let platform = gpui_kit::platform::current_platform(true);
    let text_system = TextSystem::new(platform.text_system());
    let font_id = text_system.resolve_font(&gpui_kit::font(SYSTEM_UI_FAMILY));
    let family = text_system
        .get_font_for_id(font_id)
        .map(|font| font.family.to_string())
        .unwrap_or_else(|| SYSTEM_UI_FAMILY.to_owned());
    let metrics = UiMetrics::read(&text_system, &family, size);
    (family, metrics)
}

#[cfg(test)]
#[path = "text_metrics_test.rs"]
mod text_metrics_test;
