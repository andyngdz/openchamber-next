//! The shared headless window's failure text.

use super::render_error;

#[test]
fn a_failed_headless_render_keeps_the_platform_text() {
    let error = render_error("no headless renderer is available");
    assert_eq!(
        error.to_string(),
        "cannot render the window headless: no headless renderer is available"
    );
}
