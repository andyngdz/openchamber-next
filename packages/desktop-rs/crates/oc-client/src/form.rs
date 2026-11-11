//! Pending forms (`GET /api/form`, `form.created`), their answers and number bounds.
//!
//! Shapes follow the OpenCode contract (`@opencode/client` generated `FormInfo` and
//! `FormValue`), with the `FormField` union in [`crate::form_field`]. `title` and `fields`
//! carry a default because `oc-state`'s hand-built `form.created` payloads name only the
//! form id; the captured server always sends them.

use std::collections::BTreeMap;

use serde::de::{self, Deserializer, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::form_field::FormField;

/// One pending form: a `GET /api/form` entry and the `form.created` payload.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormInfo {
    /// `frm_…` id.
    pub id: String,
    /// The asking session, or `global` for a location-scoped form.
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// The form's heading; the server always sends one.
    #[serde(default)]
    pub title: String,
    /// Server metadata; `kind` and `tool` drive the consent and recovered-question rules.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    /// The answerable fields, in the order the form asks them.
    #[serde(default)]
    pub fields: Vec<FormField>,
}

/// One choice of a picker.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FormOption {
    /// The value an answer carries.
    pub value: String,
    /// The text the picker shows.
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// A condition on an earlier answer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FormWhen {
    /// The earlier field's key.
    pub key: String,
    /// How the answers are compared.
    pub op: FormWhenOp,
    /// The value the earlier answer is compared with.
    pub value: FormScalar,
}

/// How a [`FormWhen`] compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormWhenOp {
    /// Equal.
    Eq,
    /// Not equal.
    Neq,
}

/// The formats a `string` field can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum FormFormat {
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "uri")]
    Uri,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "date-time")]
    DateTime,
}

/// A value a [`FormWhen`] compares against or an answer carries: a string, a number or a
/// boolean. The contract's `"Infinity"`, `"-Infinity"` and `"NaN"` arrive as strings.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FormScalar {
    /// A string.
    Text(String),
    /// A number.
    Number(f64),
    /// A boolean.
    Bool(bool),
}

/// One field's answer: a scalar, or the chosen values of a multiselect field.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FormValue {
    /// A single value.
    Scalar(FormScalar),
    /// The values of a multiselect answer.
    List(Vec<String>),
}

/// A form's answers, keyed by the field's `key`.
pub type FormAnswer = BTreeMap<String, FormValue>;

/// A number bound: a finite number or one of the three strings the contract allows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    /// A finite number.
    Number(f64),
    /// `"Infinity"`.
    Infinity,
    /// `"-Infinity"`.
    NegativeInfinity,
    /// `"NaN"`.
    NotANumber,
}

impl<'de> Deserialize<'de> for Bound {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BoundVisitor;

        impl Visitor<'_> for BoundVisitor {
            type Value = Bound;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a number or one of Infinity, -Infinity and NaN")
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Bound, E> {
                Ok(Bound::Number(value as f64))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Bound, E> {
                Ok(Bound::Number(value as f64))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Bound, E> {
                Ok(Bound::Number(value))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Bound, E> {
                match value {
                    "Infinity" => Ok(Bound::Infinity),
                    "-Infinity" => Ok(Bound::NegativeInfinity),
                    "NaN" => Ok(Bound::NotANumber),
                    other => Err(E::invalid_value(Unexpected::Str(other), &self)),
                }
            }
        }

        deserializer.deserialize_any(BoundVisitor)
    }
}

/// Body of `POST /api/session/:id/form/:formId/reply`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FormReplyRequest {
    /// The answers of the form's active fields.
    pub answer: FormAnswer,
}

#[cfg(test)]
#[path = "form_test.rs"]
mod form_test;
