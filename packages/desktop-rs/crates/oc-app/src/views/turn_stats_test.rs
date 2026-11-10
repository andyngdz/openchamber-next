use oc_state::State;

use super::stat_rows;
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::views::turns::turns;

fn finished_state() -> State {
    let script = StateScript::load(&state_path("session")).expect("state fixture loads");
    let mut state = State::new(script.directory);
    for action in script.actions {
        state.apply(action);
    }
    state
}

#[test]
fn rows_are_named_by_label_and_show_the_figures_the_state_holds() {
    let state = finished_state();
    let open = state.open().expect("a session is open");
    let all = turns(&open.messages);
    let rows = stat_rows(all.last().expect("one turn"));

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
        ["-", "-", "24.1s", "-", "-", "-", "21", "-", "-", "-"]
    );
}
