//! The footer trigger's CSS box.

use super::TRIGGER_HEIGHT;

#[test]
fn the_trigger_height_and_gaps_are_the_css_values() {
    // `h-8` (`ModelControls.tsx:677`), `gap-1.5` model/agent (`2358,2855`), `gap-1`
    // Thinking (`2739`), `gap-x-3` footer group (`ComposerFooter.tsx:284-288`).
    assert_eq!(TRIGGER_HEIGHT, 32.);
    assert_eq!(super::GAP_MODEL, 6.);
    assert_eq!(super::GAP_THINKING, 4.);
    assert_eq!(super::GAP_AGENT, 6.);
    assert_eq!(super::GAP_GROUP, 12.);
}
