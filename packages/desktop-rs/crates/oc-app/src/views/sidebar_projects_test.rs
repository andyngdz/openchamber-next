//! The project header actions.

use super::{NEW_DRAFT, NEW_WORKTREE};

#[test]
fn the_project_menu_offers_new_session_targets_only() {
    assert_eq!(
        [NEW_WORKTREE, NEW_DRAFT],
        ["New worktree", "New draft session"]
    );
}
