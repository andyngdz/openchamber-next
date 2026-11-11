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
    // The search row and the leading action with no list rows.
    assert_eq!(panel_height(0, 0), 66.);
    assert_eq!(panel_height(30, 4), MAX_HEIGHT);
}
