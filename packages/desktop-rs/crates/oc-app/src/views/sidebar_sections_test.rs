//! The section labels.

use super::{CHATS, IN_WORK, PINNED, PROJECTS};

#[test]
fn the_builtin_section_labels_match_react() {
    assert_eq!(
        [CHATS, PINNED, IN_WORK, PROJECTS],
        ["Chats", "Pinned", "In work", "Projects"]
    );
}
