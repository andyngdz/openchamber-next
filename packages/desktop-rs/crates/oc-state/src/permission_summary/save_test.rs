use crate::permission_summary::{can_save_permission, describe_save_patterns};

#[test]
fn offers_always_only_when_the_request_saves_something() {
    assert!(!can_save_permission(None));
    assert!(!can_save_permission(Some(&[])));
    assert!(can_save_permission(Some(&["*".to_owned()])));
    assert!(can_save_permission(Some(&["git *".to_owned()])));
}

#[test]
fn hides_a_whole_capability_save_and_strips_directory_globs() {
    assert!(describe_save_patterns("read", &["*".to_owned()]).is_empty());
    assert_eq!(
        describe_save_patterns("external_directory", &["/home/me/.config/*".to_owned()]),
        ["/home/me/.config"]
    );
    assert_eq!(
        describe_save_patterns("shell", &["git *".to_owned()]),
        ["git *"]
    );
}
