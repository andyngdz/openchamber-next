//! The model picker panel's CSS-derived size.

use super::{MAX_HEIGHT, WIDTH};

#[test]
fn the_panel_is_380_px_and_capped_at_the_reference_height() {
    // `w-[min(380px,calc(100vw-2rem))]` (`ModelControls.tsx:2407-2409`).
    assert_eq!(WIDTH, 380.);
    assert_eq!(MAX_HEIGHT, 392.6);
}
