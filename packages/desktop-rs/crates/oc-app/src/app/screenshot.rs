//! `--screenshot`: the fixture state drawn in a headless window and written as a PNG.
//!
//! The window is built the way the app builds it (`gpui_kit::open_window`, the same options,
//! the same theme, a 1440x900 content area), so the image shows what the user sees. GPUI's
//! headless renderer rasterizes the frame, so no graphical session is needed.

use std::path::Path;
use std::sync::Arc;

use gpui_kit::platform::{current_headless_renderer, current_platform};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, WindowOptions, assets, init, open_window,
    px, size,
};
use oc_state::State;

use super::{RESIZE_ATTEMPTS, SETTLE_FRAMES, WINDOW_HEIGHT, WINDOW_WIDTH, window_options};
use crate::errors::AppError;
use crate::layout::LayoutRegistry;
use crate::root::RootView;
use crate::theme::{self, DARK};
use crate::views::PopoverKind;

/// Writes the window of `state` to `output` as a PNG.
pub(super) fn render(
    output: &Path,
    state: State,
    popover: Option<PopoverKind>,
) -> Result<(), AppError> {
    let text_system = current_platform(true).text_system();
    let mut headless = HeadlessAppContext::with_platform(
        text_system,
        Arc::new(assets::Assets),
        current_headless_renderer,
    );
    // Asset loading is real I/O: the window's icons read while the frames are drawn.
    headless.allow_parking();
    let registry = LayoutRegistry::default();
    let window = open(&mut headless, state, popover, registry.clone())?;
    settle(&mut headless, window, &registry)?;
    let image = headless.capture_screenshot(window).map_err(render_error)?;
    write_png(output, &image)
}

/// Writes one frame as a PNG, whatever the output file is called.
fn write_png(output: &Path, image: &image::RgbaImage) -> Result<(), AppError> {
    image::save_buffer_with_format(
        output,
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|source| AppError::WriteScreenshot {
        path: output.to_path_buf(),
        source,
    })
}

/// Opens the app's window on the fixture state, with no server and no live updates.
fn open(
    headless: &mut HeadlessAppContext,
    state: State,
    popover: Option<PopoverKind>,
    registry: LayoutRegistry,
) -> Result<AnyWindowHandle, AppError> {
    headless
        .update(|cx| {
            init(cx);
            theme::apply(DARK, cx);
            let options = WindowOptions {
                focus: false,
                show: false,
                ..window_options(cx)
            };
            open_window(options, cx, move |window, cx| {
                cx.new(|cx| RootView::new(state, popover, registry, None, window, cx))
            })
        })
        .map(|(window, _)| window)
        .map_err(render_error)
}

/// Draws until the frame settles, growing the window while the app's content area is not the
/// reference size, as the layout dump does before it writes.
fn settle(
    headless: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    registry: &LayoutRegistry,
) -> Result<(), AppError> {
    settle_from(headless, window, registry, RESIZE_ATTEMPTS)
}

fn settle_from(
    headless: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    registry: &LayoutRegistry,
    attempts: u32,
) -> Result<(), AppError> {
    draw(headless, window, SETTLE_FRAMES)?;
    let wanted = size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT));
    let content = registry.content().map(|bounds| bounds.size);
    if let Some(content) = content.filter(|content| *content != wanted && attempts > 0) {
        let viewport = headless
            .update_window(window, |_, window, _| window.viewport_size())
            .map_err(render_error)?;
        headless
            .update_window(window, |_, window, _| {
                window.resize(viewport + wanted - content);
            })
            .map_err(render_error)?;
        return settle_from(headless, window, registry, attempts - 1);
    }
    Ok(())
}

/// Draws `frames` frames, running the queued asset work between them.
fn draw(
    headless: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    frames: u32,
) -> Result<(), AppError> {
    for _ in 0..frames {
        headless
            .update_window(window, |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .map_err(render_error)?;
        headless.run_until_parked();
    }
    Ok(())
}

fn render_error(error: impl std::fmt::Display) -> AppError {
    AppError::RenderScreenshot(error.to_string())
}

#[cfg(test)]
#[path = "screenshot_test.rs"]
mod screenshot_test;
