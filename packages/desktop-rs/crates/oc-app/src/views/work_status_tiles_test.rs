use super::UNKNOWN;

#[test]
fn a_tile_shows_a_dash_for_a_figure_the_state_lacks() {
    assert_eq!(UNKNOWN, "-");
}
