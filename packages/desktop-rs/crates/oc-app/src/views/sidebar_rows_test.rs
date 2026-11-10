//! The session row actions.

use super::{ARCHIVE, MARK_DONE, TRACK_WORK};

#[test]
fn the_row_actions_match_the_reference() {
    assert_eq!(
        [TRACK_WORK, MARK_DONE, ARCHIVE],
        ["Track as in work", "Mark done", "Archive"]
    );
}
