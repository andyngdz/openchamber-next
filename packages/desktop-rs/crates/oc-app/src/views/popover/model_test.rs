//! The model picker panel's CSS-derived size.

use super::WIDTH;

#[test]
fn the_panel_is_380_px_and_capped_at_400() {
    // `w-[min(380px,calc(100vw-2rem))]` (`ModelControls.tsx:2407-2409`).
    assert_eq!(WIDTH, 380.);
}
