use super::{INSTANCE, NEW_SESSION, RESIZE, SETTINGS, TITLE_BAR, TOOLBAR};
use crate::views::reference_test::names as reference_names;

#[test]
fn fixed_sidebar_controls_follow_the_reference_order() {
    let mut head: Vec<String> = TITLE_BAR
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    head.push(NEW_SESSION.to_owned());
    head.push(RESIZE.to_owned());
    head.extend(TOOLBAR.iter().map(|(name, _)| (*name).to_owned()));
    let tail = [SETTINGS.to_owned(), INSTANCE.to_owned()];

    for state in ["empty", "projects", "session", "streaming"] {
        let reference = reference_names(state, "sidebar");
        assert_eq!(reference[..head.len()], head[..], "{state} head");
        assert_eq!(reference[reference.len() - 2..], tail[..], "{state} tail");
    }
}
