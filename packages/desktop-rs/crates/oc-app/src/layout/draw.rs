//! What a control draws: its icons, its color marks and its text runs.
//!
//! The capture records the same three fields for the Electron reference
//! (`tools/capture-reference.mjs`, "What each control draws"). A view declares them on the
//! control it registers; the dump writes them with the control's bounds, except a mark,
//! which records its own bounds where it is painted.

use gpui_kit::{Bounds, Hsla, Pixels, SharedString};

use crate::icons::RemixIcon;
use crate::theme;

/// One icon a control draws, in DOM order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum IconDraw {
    /// A Remix sprite symbol, tinted with `color`.
    Remix { name: SharedString, color: Hsla },
    /// A ProviderLogo image; the dump names it `img:<id>` and the capture records no color.
    Image { id: SharedString },
    /// An SVG React draws itself; the dump names it `other`.
    Custom { color: Hsla },
}

impl IconDraw {
    /// The reference name: the sprite symbol, `img:<id>` or `other`.
    pub(crate) fn name(&self) -> String {
        match self {
            Self::Remix { name, .. } => name.to_string(),
            Self::Image { id } => format!("img:{id}"),
            Self::Custom { .. } => "other".to_owned(),
        }
    }

    /// The `#rrggbbaa` the capture records, if the icon has one.
    pub(crate) fn color(&self) -> Option<String> {
        match self {
            Self::Remix { color, .. } | Self::Custom { color } => Some(theme::hex(*color)),
            Self::Image { .. } => None,
        }
    }
}

/// One run of visible text and its color.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextDraw {
    /// The text after the capture's whitespace collapse.
    pub(crate) text: SharedString,
    /// The color the run is painted with.
    pub(crate) color: Hsla,
}

impl TextDraw {
    /// A run, with the capture's whitespace collapse (`\s+` to one space, trimmed).
    pub(crate) fn new(text: impl Into<SharedString>, color: Hsla) -> Self {
        Self {
            text: collapse(&text.into()).into(),
            color,
        }
    }

    /// The `#rrggbbaa` the capture records.
    pub(crate) fn color(&self) -> String {
        theme::hex(self.color)
    }
}

/// A mark: a small textless filled shape, recorded where it is painted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarkDraw {
    /// The control the mark belongs to.
    pub(crate) control: SharedString,
    /// The fill.
    pub(crate) color: Hsla,
    /// Where the shape was painted.
    pub(crate) bounds: Bounds<Pixels>,
}

/// `\s+` to one space, trimmed, as `capture-reference.mjs`'s `collapse` does.
pub(crate) fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        if character.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(character);
    }
    out
}

/// A Remix icon draw.
pub(crate) fn remix(icon: RemixIcon, color: Hsla) -> IconDraw {
    IconDraw::Remix {
        name: icon.name().into(),
        color,
    }
}

/// A provider logo draw.
pub(crate) fn image(id: impl Into<SharedString>) -> IconDraw {
    IconDraw::Image { id: id.into() }
}

#[cfg(test)]
#[path = "draw_test.rs"]
mod draw_test;
