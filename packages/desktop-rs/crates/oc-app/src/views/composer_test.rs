use oc_state::SessionStatus;

use super::{PLACEHOLDER, tool_name, toolbar};
use crate::views::reference_test::names as reference_names;

fn composer_names(status: SessionStatus) -> Vec<&'static str> {
    std::iter::once(PLACEHOLDER)
        .chain(toolbar(status).into_iter().filter_map(tool_name))
        .collect()
}

#[test]
fn composer_controls_follow_the_reference_order() {
    assert_eq!(
        composer_names(SessionStatus::Idle),
        reference_names("empty", "composer")
    );
    assert_eq!(
        composer_names(SessionStatus::Idle),
        reference_names("session", "composer")
    );
    assert_eq!(
        composer_names(SessionStatus::Busy),
        reference_names("streaming", "composer")
    );
}
