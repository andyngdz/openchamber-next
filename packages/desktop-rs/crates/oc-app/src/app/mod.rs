//! Opens the window, and in dump mode writes the layout registry after the first frames.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, WindowBounds, WindowDecorations, WindowOptions,
    application, assets, init, open_window, px, size,
};
use oc_state::{Action, State};

use crate::errors::AppError;
use crate::fixture::StateScript;
use crate::layout::{LayoutDump, LayoutRegistry};
use crate::root::RootView;
use crate::theme::{self, DARK};

/// The content size the parity reference was measured at.
const WINDOW_WIDTH: f32 = 1440.;
const WINDOW_HEIGHT: f32 = 900.;
/// Frames drawn before the dump, so text layout and the scroll position have settled.
const SETTLE_FRAMES: u32 = 3;
/// Resizes allowed while the content area converges on the reference size.
const RESIZE_ATTEMPTS: u32 = 3;

/// What the binary was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// Open the window on `directory`.
    Window {
        /// Project directory the window shows.
        directory: String,
    },
    /// Open the window on the state a fixture file builds, for review; no server.
    Fixture {
        /// A `fixtures/states/<name>.json` file.
        state: PathBuf,
    },
    /// Build the state from a fixture file, render, write the layout to `output` and exit.
    DumpLayout {
        /// Where the layout JSON goes.
        output: PathBuf,
        /// A `fixtures/states/<name>.json` file; `<name>` becomes the dump's `state`.
        state: PathBuf,
    },
}

struct DumpTarget {
    output: PathBuf,
    name: String,
}

/// Runs the app until its window closes; in dump mode, until the layout is written.
pub fn run(launch: Launch) -> Result<(), AppError> {
    match launch {
        Launch::Window { directory } => open(State::new(directory), Vec::new(), None),
        Launch::Fixture { state } => {
            let script = StateScript::load(&state)?;
            open(State::new(script.directory), script.actions, None)
        }
        Launch::DumpLayout { output, state } => {
            let script = StateScript::load(&state)?;
            let name = state_name(&state);
            let target = DumpTarget { output, name };
            open(State::new(script.directory), script.actions, Some(target))
        }
    }
}

fn state_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn open(state: State, actions: Vec<Action>, dump: Option<DumpTarget>) -> Result<(), AppError> {
    let initial = match dump {
        Some(_) => Err(AppError::DumpNotWritten),
        None => Ok(()),
    };
    let outcome = Rc::new(RefCell::new(initial));
    let final_outcome = outcome.clone();
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        theme::apply(DARK, cx);
        let registry = LayoutRegistry::default();
        let view_registry = registry.clone();
        let opened = open_window(window_options(cx), cx, move |window, cx| {
            cx.new(|cx| {
                let mut view = RootView::new(state, view_registry, window, cx);
                for action in actions {
                    view.dispatch(action, cx);
                }
                view
            })
        });
        match (opened, dump) {
            (Err(error), _) => {
                *outcome.borrow_mut() = Err(AppError::OpenWindow(error.to_string()));
                cx.quit();
                return;
            }
            (Ok((window, _)), Some(target)) => schedule_dump(window, target, registry, outcome, cx),
            (Ok(_), None) => {}
        }
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
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

fn schedule_dump(
    window: AnyWindowHandle,
    target: DumpTarget,
    registry: LayoutRegistry,
    outcome: Rc<RefCell<Result<(), AppError>>>,
    cx: &mut App,
) {
    let scheduled = window.update(cx, |_, window, _| {
        settle_then_dump(window, RESIZE_ATTEMPTS, target, registry, outcome);
    });
    if let Err(error) = scheduled {
        tracing::error!(target: crate::logging::APP, %error, "[APP] Could not schedule the layout dump");
        cx.quit();
    }
}

/// GPUI Kit draws a frame (shadow and border) around the app on client-decorated windows, so
/// the window is grown until the app's content area is the reference size, then dumped.
fn settle_then_dump(
    window: &mut gpui_kit::Window,
    attempts: u32,
    target: DumpTarget,
    registry: LayoutRegistry,
    outcome: Rc<RefCell<Result<(), AppError>>>,
) {
    after_frames(window, SETTLE_FRAMES, move |window, cx| {
        let wanted = size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT));
        let content = registry.content().map(|bounds| bounds.size);
        if let Some(content) = content.filter(|content| *content != wanted && attempts > 0) {
            let viewport = window.viewport_size();
            window.resize(viewport + wanted - content);
            settle_then_dump(window, attempts - 1, target, registry, outcome);
            return;
        }
        *outcome.borrow_mut() = write_dump(&target, window.scale_factor(), &registry);
        cx.quit();
    });
}

fn write_dump(
    target: &DumpTarget,
    scale_factor: f32,
    registry: &LayoutRegistry,
) -> Result<(), AppError> {
    let json = LayoutDump::capture(&target.name, scale_factor, registry)?.to_json()?;
    std::fs::write(&target.output, json).map_err(|source| AppError::WriteDump {
        path: target.output.clone(),
        source,
    })
}

fn after_frames(
    window: &mut gpui_kit::Window,
    remaining: u32,
    finish: impl FnOnce(&mut gpui_kit::Window, &mut App) + 'static,
) {
    window.on_next_frame(move |window, cx| {
        if remaining == 0 {
            finish(window, cx);
        } else {
            window.refresh();
            after_frames(window, remaining - 1, finish);
        }
    });
}
