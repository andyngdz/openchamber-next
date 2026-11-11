//! The screenshot mode's write contract.

use std::path::PathBuf;

use super::write_png;
use crate::errors::AppError;

/// A path under the temporary directory that no other test run uses.
fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "oc-app-screenshot-{}-{}-{name}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

#[test]
fn a_frame_is_written_as_a_png() {
    let path = temp_path("frame.png");
    let image = image::RgbaImage::from_pixel(2, 1, image::Rgba([1, 2, 3, 255]));

    write_png(&path, &image).expect("the frame writes");

    let read = image::open(&path).expect("the file decodes").to_rgba8();
    assert_eq!(read.dimensions(), (2, 1));
    assert_eq!(read.get_pixel(1, 0), &image::Rgba([1, 2, 3, 255]));
    std::fs::remove_file(path).ok();
}

#[test]
fn a_write_failure_names_the_file() {
    let path = temp_path("missing-dir/frame.png");
    let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255]));

    let error = write_png(&path, &image).expect_err("a missing directory fails the write");

    assert!(
        matches!(&error, AppError::WriteScreenshot { path: failed, .. } if failed == &path),
        "{error:?}"
    );
}
