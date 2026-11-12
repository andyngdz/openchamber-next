//! The colors a gpui-kit component paints on its own.
//!
//! A view reports a component's text and marks with the theme field the component paints
//! them with. The source line naming that field sits beside each entry.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Hsla};

/// The component-driven colors the registered controls need.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ComponentStyle {
    /// A `ListItem`'s label: the row inherits the list's foreground
    /// (`gpui-component/src/list/list_item.rs:205`, `text_color(theme.foreground)`).
    pub(crate) list_row_text: Hsla,
}

impl ComponentStyle {
    /// Reads the theme once per frame.
    pub(crate) fn read(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            list_row_text: theme.foreground,
        }
    }
}
