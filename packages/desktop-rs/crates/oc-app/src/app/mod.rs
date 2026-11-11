//! Opens the window, and in dump or screenshot mode writes what one fixture frame shows.
//!
//! The sidecar starts on the server thread and the I/O runtime on its own thread, so the
//! window opens at once on Starting; the live window loads the shared settings, the home
//! info and the global session list. Closing the last window flushes pending settings and
//! stops the sidecar before the app quits. `--dump-layout` and `--screenshot` skip the server
//! and draw a fixture's state in a headless window instead, so they need no display.

mod headless;
mod live_start;
mod quit;
mod screenshot;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::{
    App, AppContext, Bounds, WindowBounds, WindowDecorations, WindowOptions, application, assets,
    init, open_window, px, size,
};
use oc_state::State;

use crate::errors::AppError;
use crate::fixture::StateScript;
use crate::layout::{LayoutDump, LayoutRegistry};
use crate::live::ServerLink;
use crate::root::{RootView, Started};
use crate::server::ServerProcess;
use crate::theme::{self, DARK};

/// The content size the parity reference was measured at.
const WINDOW_WIDTH: f32 = 1440.;
const WINDOW_HEIGHT: f32 = 900.;
/// Frames drawn before the dump or screenshot, so text layout and the scroll position have
/// settled.
const SETTLE_FRAMES: u32 = 3;
/// Resizes allowed while the content area converges on the reference size.
const RESIZE_ATTEMPTS: u32 = 3;

/// What the binary was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// Open the window on the shared settings, with no directory argument.
    Window,
    /// Open the window on the state a fixture file builds, for review; no server.
    Fixture {
        /// A `fixtures/states/<name>.json` file.
        state: PathBuf,
    },
    /// Build the state from a fixture file, render it in a headless window, write the layout
    /// to `output` and exit.
    DumpLayout {
        /// Where the layout JSON goes.
        output: PathBuf,
        /// A `fixtures/states/<name>.json` file; `<name>` becomes the dump's `state`.
        state: PathBuf,
    },
    /// Build the state from a fixture file, render it in a headless window, write the frame
    /// to `output` as a PNG and exit.
    Screenshot {
        /// Where the PNG goes.
        output: PathBuf,
        /// A `fixtures/states/<name>.json` file.
        state: PathBuf,
    },
}

/// The server a live window talks to, and the answer of its first start.
struct LiveWindow {
    link: ServerLink,
    process: ServerProcess,
    started: Started,
}

/// What the window is opened for.
enum Mode {
    /// A fixture's state for review; no server.
    Review,
    /// The shared settings and every project's sessions from the sidecar.
    Live(LiveWindow),
}

/// Logs why the app stopped, for `main`.
pub fn log_failure(error: &AppError) {
    tracing::error!(target: crate::logging::APP, %error, "[APP] oc-app stopped");
}

/// Runs the app until its window closes; in dump mode, until the layout is written.
pub fn run(launch: Launch) -> Result<(), AppError> {
    match launch {
        Launch::Window => live_start::run_live(),
        Launch::Fixture { state } => {
            let script = StateScript::load(&state)?;
            let (state, popover) = script.into_parts();
            open(state, popover, Mode::Review)
        }
        Launch::DumpLayout { output, state } => {
            let script = StateScript::load(&state)?;
            let name = state_name(&state);
            let (state, popover) = script.into_parts();
            dump_layout(&output, &name, state, popover)
        }
        Launch::Screenshot { output, state } => {
            let script = StateScript::load(&state)?;
            let (state, popover) = script.into_parts();
            screenshot::render(&output, state, popover)
        }
    }
}

fn state_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn open(
    state: State,
    popover: Option<crate::views::PopoverKind>,
    mode: Mode,
) -> Result<(), AppError> {
    let live = match mode {
        Mode::Review => None,
        Mode::Live(live) => Some(live),
    };
    let process = live.as_ref().map(|live| live.process.clone());
    let (link, started, flush) = match live {
        Some(live) => {
            let flush = live.link.clone();
            (Some(live.link), Some(live.started), Some(flush))
        }
        None => (None, None, None),
    };
    let outcome = Rc::new(RefCell::new(Ok(())));
    let final_outcome = outcome.clone();
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        theme::apply(DARK, cx);
        let registry = LayoutRegistry::default();
        let view_registry = registry.clone();
        let opened = open_window(window_options(cx), cx, move |window, cx| {
            cx.new(|cx| {
                let mut view = RootView::new(state, popover, view_registry, link, window, cx);
                if let Some(started) = started {
                    view.await_start(started, window, cx);
                }
                view
            })
        });
        if let Err(error) = opened {
            *outcome.borrow_mut() = Err(AppError::OpenWindow(error.to_string()));
            cx.quit();
            return;
        }
        quit::quit_after_last_window(process, flush, cx);
    });
    final_outcome.replace(Ok(()))
}

fn window_options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        window_decorations: Some(WindowDecorations::Server),
        app_id: Some("openchamber-desktop-rs".to_owned()),
        ..Default::default()
    }
}

/// Renders the fixture state in the headless window and writes its layout to `output`.
fn dump_layout(
    output: &Path,
    name: &str,
    state: State,
    popover: Option<crate::views::PopoverKind>,
) -> Result<(), AppError> {
    headless::with_settled_window(state, popover, |headless, window, registry| {
        let scale_factor = headless
            .update_window(window, |_, window, _| window.scale_factor())
            .map_err(headless::render_error)?;
        write_dump(output, name, scale_factor, registry)
    })
}

fn write_dump(
    output: &Path,
    name: &str,
    scale_factor: f32,
    registry: &LayoutRegistry,
) -> Result<(), AppError> {
    let json = LayoutDump::capture(name, scale_factor, registry)?.to_json()?;
    std::fs::write(output, json).map_err(|source| AppError::WriteDump {
        path: output.to_path_buf(),
        source,
    })
}
