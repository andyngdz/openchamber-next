//! The send switch steps and the desktop Enter policy.

use oc_client::types::SessionRecord;
use serde_json::json;

use super::super::value::{EffortChoice, ModelKey, ModelPick, Selection, SelectionSource};
use super::{EnterKeys, is_same_model_ref, resolve_send_selection, should_submit_enter};
use crate::fixture_test::{model_ref, settings};

fn desired(agent: &str, model: (&str, &str), variant: Option<EffortChoice>) -> Selection {
    Selection {
        agent: Some(agent.to_owned()),
        model: Some(ModelKey::new(model.0, model.1)),
        variant,
        variant_override: None,
        agent_source: SelectionSource::Manual,
        model_source: SelectionSource::Manual,
        model_pick: ModelPick::None,
    }
}

fn record(agent: &str, model: Option<oc_client::types::ModelRef>) -> SessionRecord {
    SessionRecord {
        id: "ses_1".to_owned(),
        agent: Some(agent.to_owned()),
        model,
    }
}

#[test]
fn a_matching_agent_and_model_switch_nothing() {
    let desired = desired("build", ("perf", "stream-300cps"), None);
    let fresh = record("build", None);
    let record_model = model_ref("perf", "stream-300cps", None);
    let steps = resolve_send_selection(&desired, &fresh, Some(&record_model));
    assert_eq!(steps, super::SendSteps::default());
}

#[test]
fn a_different_agent_is_switched_before_the_prompt() {
    let desired = desired("plan", ("perf", "stream-300cps"), None);
    let fresh = record("build", None);
    let record_model = model_ref("perf", "stream-300cps", None);
    let steps = resolve_send_selection(&desired, &fresh, Some(&record_model));
    assert_eq!(steps.agent.as_deref(), Some("plan"));
    assert_eq!(steps.model, None);
}

#[test]
fn a_different_model_or_a_variant_alone_is_switched() {
    let high = desired(
        "build",
        ("perf", "stream-600cps"),
        Some(EffortChoice::Named("high".to_owned())),
    );
    let fresh = record("build", None);
    let record_model = model_ref("perf", "stream-300cps", None);
    let steps = resolve_send_selection(&high, &fresh, Some(&record_model));
    assert_eq!(
        steps.model,
        Some(model_ref("perf", "stream-600cps", Some("high")))
    );

    let low = desired(
        "build",
        ("perf", "stream-600cps"),
        Some(EffortChoice::Named("low".to_owned())),
    );
    let record_model = model_ref("perf", "stream-600cps", Some("high"));
    let steps = resolve_send_selection(&low, &record("build", None), Some(&record_model));
    assert_eq!(
        steps.model,
        Some(model_ref("perf", "stream-600cps", Some("low")))
    );
}

#[test]
fn picking_default_sends_no_variant_and_matches_the_recorded_default() {
    let desired = desired(
        "build",
        ("perf", "stream-600cps"),
        Some(EffortChoice::Default),
    );
    let steps = resolve_send_selection(
        &desired,
        &record("build", None),
        Some(&model_ref("perf", "stream-600cps", Some("high"))),
    );
    assert_eq!(steps.model, Some(model_ref("perf", "stream-600cps", None)));

    let steps = resolve_send_selection(
        &desired,
        &record("build", None),
        Some(&model_ref("perf", "stream-600cps", Some("default"))),
    );
    assert_eq!(steps.model, None);
}

#[test]
fn a_missing_record_model_is_switched() {
    let desired = desired("build", ("perf", "stream-300cps"), None);
    let steps = resolve_send_selection(&desired, &record("build", None), None);
    assert_eq!(steps.model, Some(model_ref("perf", "stream-300cps", None)));
}

#[test]
fn wire_form_compares_an_absent_variant_as_default() {
    assert!(is_same_model_ref(
        &model_ref("p", "m", None),
        &model_ref("p", "m", Some("default"))
    ));
    assert!(!is_same_model_ref(
        &model_ref("p", "m", Some("high")),
        &model_ref("p", "m", None)
    ));
}

#[test]
fn the_desktop_enter_cases_send_or_insert_a_newline() {
    let unconfigured = settings(json!({"enterToSend": false, "enterToSendConfigured": false}));
    let enabled = settings(json!({"enterToSend": true, "enterToSendConfigured": true}));
    let disabled = settings(json!({"enterToSend": false, "enterToSendConfigured": true}));
    let keys = |shift: bool, ctrl: bool, meta: bool, alt: bool| EnterKeys {
        shift,
        ctrl,
        meta,
        alt,
    };

    let cases = [
        (
            "default Enter sends",
            &unconfigured,
            keys(false, false, false, false),
            true,
        ),
        (
            "default Alt+Enter inserts a newline",
            &unconfigured,
            keys(false, false, false, true),
            false,
        ),
        (
            "default Ctrl+Alt+Enter sends",
            &unconfigured,
            keys(false, true, false, true),
            true,
        ),
        (
            "default Meta+Enter sends",
            &unconfigured,
            keys(false, false, true, false),
            true,
        ),
        (
            "default Shift+Enter inserts a newline",
            &unconfigured,
            keys(true, false, false, false),
            false,
        ),
        (
            "default Shift+Meta+Enter inserts a newline",
            &unconfigured,
            keys(true, false, true, false),
            false,
        ),
        (
            "configured enabled Enter sends",
            &enabled,
            keys(false, false, false, false),
            true,
        ),
        (
            "configured enabled Alt+Enter inserts a newline",
            &enabled,
            keys(false, false, false, true),
            false,
        ),
        (
            "configured enabled Shift+Enter inserts a newline",
            &enabled,
            keys(true, false, false, false),
            false,
        ),
        (
            "configured disabled Enter inserts a newline",
            &disabled,
            keys(false, false, false, false),
            false,
        ),
        (
            "configured disabled Shift+Enter inserts a newline",
            &disabled,
            keys(true, false, false, false),
            false,
        ),
        (
            "configured disabled Ctrl+Enter sends",
            &disabled,
            keys(false, true, false, false),
            true,
        ),
        (
            "configured disabled Ctrl+Alt+Enter sends",
            &disabled,
            keys(false, true, false, true),
            true,
        ),
        (
            "configured disabled Cmd+Shift+Enter sends",
            &disabled,
            keys(true, false, true, false),
            true,
        ),
    ];
    for (name, settings, keys, expected) in cases {
        assert_eq!(should_submit_enter(settings, keys), expected, "{name}");
    }
}
