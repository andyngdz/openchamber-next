//! The selection values: model keys, record starts and draft targets.

use oc_client::types::SessionRecord;

use super::{DraftTarget, EffortChoice, ModelKey, Selection, SelectionSource};
use crate::fixture_test::model_ref;

#[test]
fn a_model_key_uses_the_catalog_id_and_settings_spelling() {
    let key = ModelKey::new("perf", "stream-600cps");
    assert_eq!(key.provider_id(), "perf");
    assert_eq!(key.id(), "stream-600cps");
    assert_eq!(key.settings_key(), "perf/stream-600cps");
    assert_eq!(
        key.to_model_ref(Some("high")),
        model_ref("perf", "stream-600cps", Some("high"))
    );
    assert_eq!(key.to_model_ref(None).variant, None);

    assert_eq!(ModelKey::parse("perf/stream-600cps"), Some(key));
    assert_eq!(ModelKey::parse("no-slash"), None);
    assert_eq!(ModelKey::parse("/missing-provider"), None);
    assert_eq!(ModelKey::parse("missing-model/"), None);
}

#[test]
fn a_session_record_starts_the_selection_and_its_gaps_stay_auto() {
    let record = SessionRecord {
        id: "ses_1".to_owned(),
        agent: Some("plan".to_owned()),
        model: Some(model_ref("perf", "stream-600cps", Some("high"))),
    };
    let selection = Selection::from_record(&record);
    assert_eq!(selection.agent(), Some("plan"));
    assert_eq!(
        selection.model(),
        Some(&ModelKey::new("perf", "stream-600cps"))
    );
    assert_eq!(
        selection.variant(),
        Some(&EffortChoice::Named("high".to_owned()))
    );
    assert_eq!(selection.agent_source(), SelectionSource::Session);
    assert_eq!(selection.model_source(), SelectionSource::Session);

    let partial = SessionRecord {
        id: "ses_2".to_owned(),
        agent: None,
        model: Some(model_ref("perf", "stream-300cps", None)),
    };
    let selection = Selection::from_record(&partial);
    assert_eq!(selection.agent(), None);
    assert_eq!(selection.agent_source(), SelectionSource::Auto);
    assert_eq!(selection.variant(), None);
}

#[test]
fn a_record_with_the_default_variant_sends_no_variant() {
    let record = SessionRecord {
        id: "ses_1".to_owned(),
        agent: Some("build".to_owned()),
        model: Some(model_ref("perf", "stream-300cps", Some("default"))),
    };
    let selection = Selection::from_record(&record);
    assert_eq!(selection.variant(), None);
    assert_eq!(
        selection.model().map(ModelKey::settings_key).as_deref(),
        Some("perf/stream-300cps")
    );
}

#[test]
fn a_draft_target_names_its_project() {
    let chat = DraftTarget::Chat;
    assert_eq!(chat.project_id(), None);
    assert_eq!(chat.directory(), None);

    let project = DraftTarget::Project {
        id: "path_abc".to_owned(),
        directory: "/p/app".to_owned(),
    };
    assert_eq!(project.project_id(), Some("path_abc"));
    assert_eq!(project.directory(), Some("/p/app"));
}
