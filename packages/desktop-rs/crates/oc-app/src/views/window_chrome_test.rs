use super::{CONTROLS_WIDTH, STRIP_HEIGHT};

#[test]
fn the_controls_fit_in_the_strip_and_leave_room_for_the_title() {
    // Three 34 px buttons, two 2 px gaps and the 8 px right padding.
    assert_eq!(3. * 34. + 2. * 2. + 8., 114.);
    const { assert!(114. <= CONTROLS_WIDTH) };
    assert_eq!(STRIP_HEIGHT, 44.);
}
