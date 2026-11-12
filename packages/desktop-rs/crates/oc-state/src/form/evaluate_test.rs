use serde_json::json;

use crate::fixture_test::form_from;
use crate::form::{ExternalLinks, initial_form_values, missing_required_keys};

#[test]
fn hidden_fields_contribute_no_answer() {
    let form = form_from(json!({
        "id": "frm_test",
        "sessionID": "ses_test",
        "fields": [
            {"key": "a", "type": "boolean", "default": false},
            {"key": "b", "type": "string", "required": true, "when": [{"key": "a", "op": "eq", "value": true}]}
        ]
    }));
    let values = initial_form_values(&form.fields, ExternalLinks::Unacknowledged);
    assert!(missing_required_keys(&form.fields, &values).is_empty());
}
