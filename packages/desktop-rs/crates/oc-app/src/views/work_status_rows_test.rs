use std::collections::HashSet;

use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::views::turn_stats::stat_rows;
use crate::views::turns::turns;

/// A row's label is its control name, so two rows with one label would share a key.
#[test]
fn stat_row_labels_are_unique() {
    let state = StateScript::load(&state_path("session"))
        .expect("state fixture loads")
        .state();
    let open = state.open().expect("a session is open");
    let all = turns(&open.messages);
    let rows = stat_rows(all.last().expect("one turn"));

    let labels: HashSet<&str> = rows.iter().map(|row| row.label).collect();

    assert_eq!(labels.len(), rows.len());
}
