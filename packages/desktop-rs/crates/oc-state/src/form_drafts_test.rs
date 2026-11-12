use serde_json::json;

use crate::fixture_test::form_from;
use crate::form::FormValues;
use crate::form_drafts::{FormDraft, FormDraftScope, FormDrafts, fields_signature};

fn draft(fields: &[oc_client::form_field::FormField], step: usize) -> FormDraft {
    FormDraft {
        fields_signature: fields_signature(fields),
        values: FormValues::new(),
        step,
    }
}

#[test]
fn restores_only_while_the_field_signature_matches() {
    let form = form_from(json!({
        "id": "frm_1",
        "sessionID": "ses_1",
        "fields": [
            {"key": "q0", "type": "string"},
            {"key": "q1", "type": "multiselect", "options": [{"value": "a", "label": "A"}]}
        ]
    }));
    let scope = FormDraftScope::new("/repo", "ses_1", "frm_1");
    let mut drafts = FormDrafts::default();
    drafts.save(&scope, draft(&form.fields, 1));
    assert!(drafts.load(&scope).expect("saved").matches(&form.fields));

    let other = form_from(json!({
        "id": "frm_1",
        "sessionID": "ses_1",
        "fields": [{"key": "q0", "type": "number"}]
    }));
    assert!(!drafts.load(&scope).expect("saved").matches(&other.fields));
}

#[test]
fn keeps_at_most_fifty_drafts_and_evicts_the_oldest() {
    let form = form_from(
        json!({"id": "frm_1", "sessionID": "ses_1", "fields": [{"key": "q0", "type": "string"}]}),
    );
    let mut drafts = FormDrafts::default();
    for index in 0..60 {
        let scope = FormDraftScope::new("/repo", "ses_1", format!("frm_{index}"));
        drafts.save(&scope, draft(&form.fields, index));
    }
    assert_eq!(drafts.len(), 50);
    assert!(
        drafts
            .load(&FormDraftScope::new("/repo", "ses_1", "frm_0"))
            .is_none()
    );
    assert!(
        drafts
            .load(&FormDraftScope::new("/repo", "ses_1", "frm_10"))
            .is_some()
    );
}

#[test]
fn saving_an_existing_draft_moves_it_to_the_newest_position() {
    let form = form_from(
        json!({"id": "frm_1", "sessionID": "ses_1", "fields": [{"key": "q0", "type": "string"}]}),
    );
    let mut drafts = FormDrafts::default();
    for index in 0..50 {
        let scope = FormDraftScope::new("/repo", "ses_1", format!("frm_{index}"));
        drafts.save(&scope, draft(&form.fields, index));
    }
    let first = FormDraftScope::new("/repo", "ses_1", "frm_0");
    drafts.save(&first, draft(&form.fields, 9));
    let newest = FormDraftScope::new("/repo", "ses_1", "frm_50");
    drafts.save(&newest, draft(&form.fields, 0));
    assert!(drafts.load(&first).is_some());
    assert!(
        drafts
            .load(&FormDraftScope::new("/repo", "ses_1", "frm_1"))
            .is_none()
    );
}

#[test]
fn dropping_a_draft_forgets_it() {
    let form = form_from(
        json!({"id": "frm_1", "sessionID": "ses_1", "fields": [{"key": "q0", "type": "string"}]}),
    );
    let scope = FormDraftScope::new("/repo", "ses_1", "frm_1");
    let mut drafts = FormDrafts::default();
    drafts.save(&scope, draft(&form.fields, 0));
    drafts.drop_draft(&scope);
    assert!(drafts.is_empty());
}
