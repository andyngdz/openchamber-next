//! React's own SVG icons, copied into `assets/icons/custom/`.
//!
//! These components draw an inline `<svg>` rather than a sprite symbol, so the capture names
//! them `other`. `tools/gen-remix-icons.mjs` writes the files from the React components and a
//! test keeps them equal.

use gpui_kit::SharedString;
use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, Window};

/// One custom icon's file stem and embedded SVG.
struct CustomFile {
    name: &'static str,
    bytes: &'static [u8],
}

/// Every custom icon's file.
const FILES: &[CustomFile] = &[
    CustomFile {
        name: "stop",
        bytes: asset_bytes!("/assets/icons/custom/stop.svg"),
    },
    CustomFile {
        name: "context-ring",
        bytes: asset_bytes!("/assets/icons/custom/context-ring.svg"),
    },
    CustomFile {
        name: "arrows-merge",
        bytes: asset_bytes!("/assets/icons/custom/arrows-merge.svg"),
    },
];

/// One inline SVG React draws (`StopIcon`, `ContextRing`, `ArrowsMerge`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoElement)]
pub(crate) enum CustomIcon {
    Stop,
    ContextRing,
    ArrowsMerge,
}

impl CustomIcon {
    /// Every custom icon.
    pub(crate) const ALL: &[CustomIcon] = &[Self::Stop, Self::ContextRing, Self::ArrowsMerge];

    fn index(self) -> usize {
        match self {
            Self::Stop => 0,
            Self::ContextRing => 1,
            Self::ArrowsMerge => 2,
        }
    }

    /// The file stem under `assets/icons/custom/`.
    pub(crate) fn name(self) -> &'static str {
        FILES[self.index()].name
    }

    /// The embedded SVG.
    pub(crate) fn bytes(self) -> &'static [u8] {
        FILES[self.index()].bytes
    }
}

impl IconNamed for CustomIcon {
    fn path(self) -> SharedString {
        format!("icons/custom/{}.svg", self.name()).into()
    }
}

impl RenderOnce for CustomIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}
