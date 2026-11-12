use crate::permission_patterns::get_visible_permission_patterns;

#[test]
fn omits_a_pattern_already_rendered_as_the_command() {
    let command = "bunx eslint \"src/components/session/SessionSidebar.tsx\"";
    assert!(get_visible_permission_patterns(&[command.to_owned()], command).is_empty());
}

#[test]
fn preserves_distinct_permission_patterns() {
    let command = "bunx eslint \"src/components/session/SessionSidebar.tsx\"";
    assert_eq!(
        get_visible_permission_patterns(&["bunx eslint *".to_owned(), command.to_owned()], command),
        ["bunx eslint *"]
    );
}
