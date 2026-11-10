//! Turning a popup row into a selection action.

use oc_state::SelectionAction;

use super::action_for;
use crate::views::Pick;

#[test]
fn a_project_pick_carries_the_supplied_time() {
    assert_eq!(
        action_for(Pick::Project("p".to_owned()), 42),
        SelectionAction::PickProject {
            id: "p".to_owned(),
            now: 42
        }
    );
}

#[test]
fn a_default_variant_pick_stays_none() {
    assert_eq!(
        action_for(Pick::Variant(None), 0),
        SelectionAction::PickVariant(None)
    );
}
