//! The web-search consent card's shape (`lib/opencode/websearch.ts:352-361`).
//!
//! A form whose `metadata.kind` is `websearch.provider` and whose only field is a string
//! keyed `choice` or `provider` with options renders as the consent card instead of the
//! stepper. Every other form returns `None`.

use oc_client::form_field::FormField;

use crate::requests::METADATA_KIND;

/// One consent option as the card labels it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchConsentOption {
    /// The value the reply carries.
    pub value: String,
    /// The label the button shows.
    pub label: String,
}

/// Which consent step the form asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSearchConsentStep {
    /// Allow, Choose provider, or Turn off web search.
    Choice,
    /// One button per provider.
    Provider,
}

/// The consent card a form becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchConsent {
    /// Which step the form asks.
    pub step: WebSearchConsentStep,
    /// The field the reply is keyed by.
    pub field_key: String,
    /// The buttons, in the order the form lists them.
    pub options: Vec<WebSearchConsentOption>,
}

/// `None` for any other form, or one whose shape this card does not know.
pub fn read_web_search_consent(form: &oc_client::form::FormInfo) -> Option<WebSearchConsent> {
    if form
        .metadata
        .get(METADATA_KIND)
        .and_then(|kind| kind.as_str())
        != Some("websearch.provider")
    {
        return None;
    }
    if form.fields.len() != 1 {
        return None;
    }
    let FormField::String(field) = &form.fields[0] else {
        return None;
    };
    let options = field
        .options
        .as_ref()
        .filter(|options| !options.is_empty())?;
    let step = match field.key.as_str() {
        "choice" => WebSearchConsentStep::Choice,
        "provider" => WebSearchConsentStep::Provider,
        _ => return None,
    };
    Some(WebSearchConsent {
        step,
        field_key: field.key.clone(),
        options: options
            .iter()
            .map(|option| WebSearchConsentOption {
                value: option.value.clone(),
                label: option.label.clone(),
            })
            .collect(),
    })
}

#[cfg(test)]
#[path = "websearch_test.rs"]
mod websearch_test;
