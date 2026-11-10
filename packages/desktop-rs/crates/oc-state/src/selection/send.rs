//! What a send must switch first, and how the composer decides Enter.
//!
//! `resolveSendSelection` (`sync/session-selection.ts:21-50`) compares the agent with a fresh
//! session record and the model with the record held in state, in wire form: a model with
//! no variant equals one OpenCode recorded as `"default"`. `shouldSubmitEnter`
//! (`composer/keyboardPolicy.ts:13-25`) is ported for the desktop composer that is not
//! expanded, where Ctrl and Meta count alike.

use oc_client::settings::Settings;
use oc_client::types::ModelRef;
use oc_client::types::SessionRecord;

use super::value::{DEFAULT_VARIANT, Selection};

/// The requests a send runs before the prompt; an absent step needs no call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendSteps {
    /// `POST /api/session/:id/agent` when set.
    pub agent: Option<String>,
    /// `POST /api/session/:id/model` when set; no `variant` means Default.
    pub model: Option<ModelRef>,
}

/// The agent and model a send must switch the session to.
pub fn resolve_send_selection(
    desired: &Selection,
    fresh: &SessionRecord,
    record_model: Option<&ModelRef>,
) -> SendSteps {
    let agent = desired
        .agent()
        .filter(|agent| fresh.agent.as_deref() != Some(*agent))
        .map(str::to_owned);
    let model = desired.model().and_then(|key| {
        let next = key.to_model_ref(desired.variant().and_then(|choice| choice.variant()));
        let changed = record_model.is_none_or(|current| !is_same_model_ref(current, &next));
        changed.then_some(next)
    });
    SendSteps { agent, model }
}

/// Whether a switch to `next` would leave the session on `current`.
///
/// OpenCode records a model switched without a variant as `"default"` and treats the two
/// as the same effort (`lib/opencode/model.ts:96-100`).
pub fn is_same_model_ref(current: &ModelRef, next: &ModelRef) -> bool {
    current.provider_id == next.provider_id
        && current.id == next.id
        && current.variant.as_deref().unwrap_or(DEFAULT_VARIANT)
            == next.variant.as_deref().unwrap_or(DEFAULT_VARIANT)
}

/// The modifier keys of one Enter press.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnterKeys {
    /// Shift held.
    pub shift: bool,
    /// Ctrl held.
    pub ctrl: bool,
    /// Meta (Cmd) held.
    pub meta: bool,
    /// Alt held.
    pub alt: bool,
}

/// Whether this Enter press sends, for the desktop composer that is not expanded.
///
/// With `enterToSendConfigured` false, or `enterToSend` true, Enter sends and Shift+Enter
/// or Alt+Enter inserts a newline; with `enterToSend` false, Enter inserts a newline and
/// Ctrl+Enter or Meta+Enter sends.
pub fn should_submit_enter(settings: &Settings, keys: EnterKeys) -> bool {
    let command_enter = keys.ctrl || keys.meta;
    if !settings.enter_to_send_configured {
        return !keys.shift && (!keys.alt || command_enter);
    }
    let sends_with_enter = settings.enter_to_send && !keys.shift && !keys.alt;
    command_enter || sends_with_enter
}

#[cfg(test)]
#[path = "send_test.rs"]
mod send_test;
