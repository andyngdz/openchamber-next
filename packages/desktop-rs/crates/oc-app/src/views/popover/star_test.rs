//! The star a picker row carries.

use super::{Star, StarKind};
use oc_state::{ModelEntry, SelectionAction};

fn entry() -> ModelEntry {
    ModelEntry {
        provider_id: "perf".to_owned(),
        model_id: "stream-300cps".to_owned(),
        name: "Fixture stream".to_owned(),
        provider_name: "Performance fixture".to_owned(),
        variants: Vec::new(),
        context: None,
    }
}

#[test]
fn a_starred_model_offers_unfavorite() {
    let star = Star::model(&entry(), StarKind::Filled);

    assert_eq!(star.label, "Unfavorite");
    assert!(matches!(star.kind, StarKind::Filled));
    assert!(star.shown);
    assert!(matches!(
        star.action,
        SelectionAction::ToggleFavoriteModel { .. }
    ));
}

#[test]
fn an_unstarred_model_offers_favorite() {
    let star = Star::model(&entry(), StarKind::Outline);

    assert_eq!(star.label, "Favorite");
    assert!(matches!(star.kind, StarKind::Outline));
    assert!(!star.shown);
}
