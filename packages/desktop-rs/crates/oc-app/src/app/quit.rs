//! Closing the last window: flush pending settings, stop the sidecar, then quit.

use std::time::Duration;

use gpui_kit::App;

use crate::live::ServerLink;
use crate::server::ServerProcess;

/// How long the last window's pending settings write may take before the sidecar stops.
const CLOSE_FLUSH_WAIT: Duration = Duration::from_secs(3);

/// Closing the last window flushes pending settings, stops the sidecar, then quits.
pub(super) fn quit_after_last_window(
    process: Option<ServerProcess>,
    flush: Option<ServerLink>,
    cx: &mut App,
) {
    cx.on_window_closed(move |cx, _| {
        if cx.windows().is_empty() {
            if let Some(flush) = flush.as_ref() {
                flush.flush_settings(CLOSE_FLUSH_WAIT);
            }
            if let Some(process) = process.as_ref() {
                process.stop();
            }
            cx.quit();
        }
    })
    .detach();
}
