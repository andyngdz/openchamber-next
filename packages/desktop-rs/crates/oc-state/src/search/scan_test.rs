//! The Bitap scan's score arithmetic.

use super::accuracy;

#[test]
fn accuracy_is_the_share_of_the_pattern_allowed_to_be_wrong() {
    assert_eq!(accuracy(4, 0), 0.0);
    assert_eq!(accuracy(4, 1), 0.25);
    assert_eq!(accuracy(9, 2), 2.0 / 9.0);
    assert_eq!(accuracy(3, 3), 1.0);
}
