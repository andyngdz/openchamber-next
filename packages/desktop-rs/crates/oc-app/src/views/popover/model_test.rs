//! The model picker panel's CSS-derived size.

use super::{MAX_HEIGHT, WIDTH, panel_height};

#[test]
fn the_panel_is_380_px_and_capped_at_the_reference_height() {
    // `w-[min(380px,calc(100vw-2rem))]` (`ModelControls.tsx:2407-2409`).
    assert_eq!(WIDTH, 380.);
    assert_eq!(MAX_HEIGHT, 392.6);
}

#[test]
fn the_panel_grows_with_its_rows_and_stops_at_the_cap() {
    // The 40 px search row, its divider, the list's 4 px padding, the leading action and the
    // 32 px footer under its divider, with no list rows.
    assert_eq!(panel_height(0, 0), 108.);
    assert_eq!(panel_height(30, 4), MAX_HEIGHT);
}
