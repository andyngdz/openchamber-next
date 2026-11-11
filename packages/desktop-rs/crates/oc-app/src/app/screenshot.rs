//! `--screenshot`: the fixture state drawn in a headless window and written as a PNG.
//!
//! The frame comes from the same headless window `--dump-layout` measures (`headless.rs`), so
//! the image and the dump show one layout.

use std::path::Path;

use oc_state::State;

use super::headless::{self, render_error};
use crate::errors::AppError;
use crate::views::PopoverKind;

/// Writes the window of `state` to `output` as a PNG.
pub(super) fn render(
    output: &Path,
    state: State,
    popover: Option<PopoverKind>,
) -> Result<(), AppError> {
    let image = headless::with_settled_window(state, popover, |headless, window, _| {
        headless.capture_screenshot(window).map_err(render_error)
    })?;
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

#[cfg(test)]
#[path = "screenshot_test.rs"]
mod screenshot_test;
