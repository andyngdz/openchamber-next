use oc_state::State;

use super::stat_rows;
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::views::turns::turns;

fn finished_state() -> State {
    StateScript::load(&state_path("session"))
        .expect("state fixture loads")
        .state()
}

#[test]
fn rows_are_named_by_label_and_show_the_figures_the_state_holds() {
    let state = finished_state();
    let open = state.open().expect("a session is open");
    let all = turns(&open.messages);
    let turn = all.last().expect("one turn");
    // The elapsed figure is the fixture's own turn duration, not a number the
    // test keeps in step with the capture by hand.
    let elapsed = turn.meta().expect("a finished turn").duration;
    let rows = stat_rows(turn);

    let labels: Vec<&str> = rows.iter().map(|row| row.label).collect();
    assert_eq!(
        labels,
        [
            "Response",
            "Whole turn",
            "Elapsed",
            "Model time",
            "Tool time",
            "Average TTFT",
            "Steps",
            "Tokens",
            "Cache",
            "Cost",
        ]
    );
    let values: Vec<&str> = rows.iter().map(|row| row.value.as_str()).collect();
    assert_eq!(
        values,
        [
            "-",
            "-",
            elapsed.as_str(),
            "-",
            "-",
            "-",
            "21",
            "-",
            "-",
            "-",
        ]
    );
}
