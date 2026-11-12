//! The tagged `FormField` union of `GET /api/form` and `form.created`.
//!
//! Shapes follow the OpenCode contract (`@opencode/client` generated `FormField`), tagged
//! by `type`. A field kind this client does not know fails the form's decode, as React's
//! schema does.

use serde::Deserialize;

use crate::form::{Bound, FormFormat, FormOption, FormWhen};

/// One answerable field, tagged by `type`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FormField {
    /// Free text, or a choice list when `options` is set.
    String(FormStringField),
    /// A decimal number.
    Number(FormNumberField),
    /// A whole number.
    Integer(FormIntegerField),
    /// Yes or no.
    Boolean(FormBooleanField),
    /// A list of chosen option values.
    Multiselect(FormMultiselectField),
    /// A link the user must open before submitting.
    External(FormExternalField),
}

/// A `string` field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormStringField {
    /// The answer key.
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub hidden: bool,
    /// When the field applies, compared against earlier active answers.
    #[serde(default)]
    pub when: Vec<FormWhen>,
    #[serde(default)]
    pub format: Option<FormFormat>,
    #[serde(default, rename = "minLength")]
    pub min_length: Option<u64>,
    #[serde(default, rename = "maxLength")]
    pub max_length: Option<u64>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
    /// The choices, when this field is a picker.
    #[serde(default)]
    pub options: Option<Vec<FormOption>>,
    /// Whether the picker accepts a typed "Other" answer.
    #[serde(default)]
    pub custom: bool,
}

/// A `number` field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormNumberField {
    /// The answer key.
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub when: Vec<FormWhen>,
    #[serde(default)]
    pub minimum: Option<Bound>,
    #[serde(default)]
    pub maximum: Option<Bound>,
    #[serde(default)]
    pub default: Option<Bound>,
}

/// An `integer` field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormIntegerField {
    /// The answer key.
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub when: Vec<FormWhen>,
    #[serde(default)]
    pub minimum: Option<Bound>,
    #[serde(default)]
    pub maximum: Option<Bound>,
    #[serde(default)]
    pub default: Option<Bound>,
}

/// A `boolean` field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormBooleanField {
    /// The answer key.
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub when: Vec<FormWhen>,
    #[serde(default)]
    pub default: Option<bool>,
}

/// A `multiselect` field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormMultiselectField {
    /// The answer key.
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub when: Vec<FormWhen>,
    /// The choices.
    pub options: Vec<FormOption>,
    #[serde(default, rename = "minItems")]
    pub min_items: Option<u64>,
    #[serde(default, rename = "maxItems")]
    pub max_items: Option<u64>,
    /// Whether the picker accepts a typed "Other" answer.
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub default: Vec<String>,
}

/// An `external` field: a link, not an answer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormExternalField {
    /// The answer key.
    pub key: String,
    /// Where the link goes.
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl FormField {
    /// The field's answer key.
    pub fn key(&self) -> &str {
        match self {
            Self::String(field) => &field.key,
            Self::Number(field) => &field.key,
            Self::Integer(field) => &field.key,
            Self::Boolean(field) => &field.key,
            Self::Multiselect(field) => &field.key,
            Self::External(field) => &field.key,
        }
    }

    /// The contract's `type` tag.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Number(_) => "number",
            Self::Integer(_) => "integer",
            Self::Boolean(_) => "boolean",
            Self::Multiselect(_) => "multiselect",
            Self::External(_) => "external",
        }
    }
}
