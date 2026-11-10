//! Closing the last window: flush pending settings, stop the sidecar, then quit.

use gpui_kit::App;

use crate::live::{SETTINGS_FLUSH_WAIT, ServerLink};
use crate::server::ServerProcess;

/// Closing the last window flushes pending settings, stops the sidecar, then quits.
pub(super) fn quit_after_last_window(
    process: Option<ServerProcess>,
    flush: Option<ServerLink>,
    cx: &mut App,
) {
    cx.on_window_closed(move |cx, _| {
        if cx.windows().is_empty() {
            if let Some(flush) = flush.as_ref() {
                flush.flush_settings(SETTINGS_FLUSH_WAIT);
            }
            if let Some(process) = process.as_ref() {
                process.stop();
            }
            cx.quit();
        }
    })
    .detach();
}
