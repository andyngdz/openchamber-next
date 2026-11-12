//! Memory-only form answers that survive switching between a question's surfaces
//! (`components/chat/formDrafts.ts`).
//!
//! At most 50 drafts are kept; the oldest is evicted. A draft restores only while the
//! field signature is unchanged, so a rebuilt list with the same content keeps the answers
//! and a different form starts over.

use std::collections::{BTreeMap, VecDeque};

use oc_client::form_field::FormField;

use crate::form::FormValues;

/// React's cap on drafts kept in memory (`formDrafts.ts:20`).
const MAX_FORM_DRAFTS: usize = 50;

/// Where a draft belongs: one runtime, directory, session and form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormDraftScope {
    /// The server runtime the form belongs to; one app runtime is `""`.
    pub runtime_key: String,
    /// The form's directory.
    pub directory: String,
    /// The asking session.
    pub session_id: String,
    /// The form id.
    pub form_id: String,
}

impl FormDraftScope {
    /// A scope of this app's one runtime.
    pub fn new(
        directory: impl Into<String>,
        session_id: impl Into<String>,
        form_id: impl Into<String>,
    ) -> Self {
        Self {
            runtime_key: String::new(),
            directory: directory.into(),
            session_id: session_id.into(),
            form_id: form_id.into(),
        }
    }

    fn key(&self) -> String {
        [
            self.runtime_key.as_str(),
            self.directory.as_str(),
            self.session_id.as_str(),
            self.form_id.as_str(),
        ]
        .join("\u{0}")
    }
}

/// The answers and step kept for one form.
#[derive(Debug, Clone, PartialEq)]
pub struct FormDraft {
    /// `key:type` per field, joined by `|`.
    pub fields_signature: String,
    /// The answers so far.
    pub values: FormValues,
    /// The current step.
    pub step: usize,
}

impl FormDraft {
    /// Whether the draft still belongs to this field list.
    pub fn matches(&self, fields: &[FormField]) -> bool {
        self.fields_signature == fields_signature(fields)
    }
}

/// The field list's identity, as `FormDock.tsx:119` builds it.
pub fn fields_signature(fields: &[FormField]) -> String {
    let mut signature = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            signature.push('|');
        }
        signature.push_str(field.key());
        signature.push(':');
        signature.push_str(field.kind());
    }
    signature
}

/// The in-memory drafts, oldest evicted first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormDrafts {
    drafts: BTreeMap<String, FormDraft>,
    order: VecDeque<String>,
}

impl FormDrafts {
    /// The draft of `scope`, when one was saved.
    pub fn load(&self, scope: &FormDraftScope) -> Option<&FormDraft> {
        self.drafts.get(&scope.key())
    }

    /// Saves a draft; a replaced draft moves to the newest position.
    pub fn save(&mut self, scope: &FormDraftScope, draft: FormDraft) {
        let key = scope.key();
        self.drafts.remove(&key);
        self.order.retain(|held| held != &key);
        self.drafts.insert(key.clone(), draft);
        self.order.push_back(key);
        while self.order.len() > MAX_FORM_DRAFTS {
            if let Some(oldest) = self.order.pop_front() {
                self.drafts.remove(&oldest);
            }
        }
    }

    /// Drops a draft once its form is submitted or dismissed.
    pub fn drop_draft(&mut self, scope: &FormDraftScope) {
        let key = scope.key();
        self.drafts.remove(&key);
        self.order.retain(|held| held != &key);
    }

    /// How many drafts are kept.
    pub fn len(&self) -> usize {
        self.drafts.len()
    }

    /// No draft is kept.
    pub fn is_empty(&self) -> bool {
        self.drafts.is_empty()
    }
}

#[cfg(test)]
#[path = "form_drafts_test.rs"]
mod form_drafts_test;
