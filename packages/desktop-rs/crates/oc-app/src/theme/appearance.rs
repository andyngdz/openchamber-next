//! Follows the OS light or dark appearance, so both bundled themes are reachable.

use gpui_kit::{App, Window, WindowAppearance};

use super::{Mode, tokens};

/// The theme mode that matches an OS window appearance.
pub(super) fn mode_for(appearance: WindowAppearance) -> Mode {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => Mode::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Mode::Dark,
    }
}

/// Applies the tokens for the window's current appearance, and again whenever it changes.
pub(crate) fn follow_system(window: &mut Window, cx: &mut App) {
    tokens(mode_for(window.appearance())).apply(cx);
    window
        .observe_window_appearance(|window, cx| {
            tokens(mode_for(window.appearance())).apply(cx);
            window.refresh();
        })
        .detach();
}

#[cfg(test)]
#[path = "appearance_test.rs"]
mod appearance_test;
