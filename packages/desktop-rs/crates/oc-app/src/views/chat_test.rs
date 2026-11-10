use super::COMPOSER_BOTTOM;

#[test]
fn the_open_session_composer_ends_16_px_above_the_window_edge() {
    // Reference: composer y 784, height 100, window height 900.
    assert!((900. - (784. + 100.) - COMPOSER_BOTTOM).abs() < f32::EPSILON);
}
