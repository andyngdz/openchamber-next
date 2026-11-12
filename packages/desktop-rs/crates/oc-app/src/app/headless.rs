//! The headless window both `--dump-layout` and `--screenshot` render through.
//!
//! The window is built the way the app builds it (`gpui_kit::open_window`, the same options,
//! theme and 1440x900 content area), but in GPUI's headless context with the real text system
//! and the headless renderer, so neither mode opens a visible window. Both modes draw the same
//! settled frame, so a dump and a screenshot show the same layout.

use std::sync::Arc;

use gpui_kit::platform::{current_headless_renderer, current_platform};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, WindowOptions, init, open_window, px,
    size,
};
use oc_state::State;

use super::{RESIZE_ATTEMPTS, SETTLE_FRAMES, WINDOW_HEIGHT, WINDOW_WIDTH, window_options};
use crate::AppError;
use crate::layout::LayoutRegistry;
use crate::root::RootView;
use crate::theme;
use crate::views::PopoverKind;

/// Draws `state` in a headless window until the frame settles, then hands the window and its
/// registry to `finish`.
pub(crate) fn with_settled_window<T>(
    state: State,
    popover: Option<PopoverKind>,
    finish: impl FnOnce(
        &mut HeadlessAppContext,
        AnyWindowHandle,
        &LayoutRegistry,
    ) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let text_system = current_platform(true).text_system();
    let mut headless = HeadlessAppContext::with_platform(
        text_system,
        Arc::new(crate::icons::AppAssets),
        current_headless_renderer,
    );
    // Asset loading is real I/O: the window's icons read while the frames are drawn.
    headless.allow_parking();
    let registry = LayoutRegistry::default();
    let window = open(&mut headless, state, popover, registry.clone())?;
    settle(&mut headless, window, &registry)?;
    finish(&mut headless, window, &registry)
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
            theme::apply(theme::palette(), cx);
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
/// reference size.
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

/// The error text a headless window failure gets.
pub(super) fn render_error(error: impl std::fmt::Display) -> AppError {
    AppError::RenderHeadless(error.to_string())
}

#[cfg(test)]
#[path = "headless_test.rs"]
mod headless_test;

#[cfg(test)]
#[path = "text_fit_test.rs"]
mod text_fit_test;
