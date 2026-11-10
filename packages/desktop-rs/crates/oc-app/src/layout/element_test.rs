use super::resting_opacity;
use crate::layout::registry::Reveal;

#[test]
fn hover_only_and_hidden_controls_rest_transparent() {
    assert!((resting_opacity(&Reveal::Always) - 1.).abs() < f32::EPSILON);
    assert!(resting_opacity(&Reveal::OnHover("row".into())).abs() < f32::EPSILON);
    assert!(resting_opacity(&Reveal::Hidden).abs() < f32::EPSILON);
}
