//! The popup row values.

use super::Pick;

#[test]
fn a_variant_pick_keeps_default_as_none() {
    assert_eq!(Pick::Variant(None), Pick::Variant(None));
    assert_ne!(Pick::Variant(None), Pick::Variant(Some("high".to_owned())));
}
