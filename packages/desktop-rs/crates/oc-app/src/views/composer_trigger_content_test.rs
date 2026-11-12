//! The trigger content's lookup keys and logo slot.

use super::{content_selector, label_selector};
use crate::views::composer::slots::TriggerKind;

const LAYOUTS: [TriggerKind; 3] = [
    TriggerKind::Model,
    TriggerKind::Thinking,
    TriggerKind::Agent,
];

#[test]
fn every_trigger_has_its_own_row_and_label_keys() {
    let mut keys: Vec<&str> = LAYOUTS
        .iter()
        .flat_map(|layout| [content_selector(*layout), label_selector(*layout)])
        .collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), LAYOUTS.len() * 2);
}

#[test]
fn every_label_key_sits_under_its_row_key() {
    for layout in LAYOUTS {
        assert!(label_selector(layout).starts_with(content_selector(layout)));
    }
}
