//! Pure form logic behind the form dock (`components/chat/formCardState.ts`).
//!
//! Answers, defaults and `when` clauses are evaluated here, so the dock stays a view. Only
//! required fields, acknowledged links and parseable numbers are enforced; bounds, item
//! counts, patterns and formats are the server's to reject.

use oc_client::form::{Bound, FormAnswer, FormScalar, FormValue};
use oc_client::form_field::{FormField, FormStringField};

/// Whether external fields start acknowledged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalLinks {
    /// The stepped dock acknowledges each link as its step opens.
    Unacknowledged,
    /// A surface that shows every field at once has already shown the link.
    Acknowledged,
}

/// One field's value in the dock.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldValue {
    /// Typed text, also the "Other" answer of a picker.
    pub text: String,
    /// A number or integer answer.
    pub number: Option<f64>,
    /// A yes/no answer.
    pub boolean: bool,
    /// Picked option values.
    pub selected: Vec<String>,
    /// The user typed their own answer instead of picking one.
    pub custom: bool,
    /// An external link has been opened.
    pub acknowledged: bool,
}

/// Every field's value, keyed by the field's `key`.
pub type FormValues = std::collections::BTreeMap<String, FieldValue>;

/// What the form resolves to right now.
#[derive(Debug, Clone, PartialEq)]
pub struct FormEvaluation {
    /// Fields shown to the user, in declaration order.
    pub active: Vec<FormField>,
    /// The reply so far.
    pub answer: FormAnswer,
}

static EMPTY_FIELD: FieldValue = FieldValue {
    text: String::new(),
    number: None,
    boolean: false,
    selected: Vec::new(),
    custom: false,
    acknowledged: false,
};

/// The value a field currently holds, empty when it has none.
pub fn value_of<'a>(values: &'a FormValues, key: &str) -> &'a FieldValue {
    values.get(key).unwrap_or(&EMPTY_FIELD)
}

/// Fields whose value the user edits; `external` only links out.
pub fn is_answerable_field(field: &FormField) -> bool {
    !matches!(field, FormField::External(_))
}

/// The finite number a bound or default stands for; the JSON stand-ins read as absent.
pub(crate) fn finite(bound: Option<Bound>) -> Option<f64> {
    match bound {
        Some(Bound::Number(number)) if number.is_finite() => Some(number),
        Some(_) | None => None,
    }
}

/// The dock's starting state: every field seeded from its declared default
/// (`formCardState.ts:67-102`).
pub fn initial_form_values(fields: &[FormField], links: ExternalLinks) -> FormValues {
    let mut values = FormValues::new();
    for field in fields {
        let mut field_value = FieldValue::default();
        match field {
            FormField::External(_) => {
                field_value.acknowledged = links == ExternalLinks::Acknowledged;
            }
            FormField::String(field) => seed_string(&mut field_value, field),
            FormField::Number(field) => field_value.number = finite(field.default),
            FormField::Integer(field) => field_value.number = finite(field.default),
            FormField::Boolean(field) => field_value.boolean = field.default.unwrap_or(false),
            FormField::Multiselect(field) => {
                field_value.selected.clone_from(&field.default);
            }
        }
        values.insert(field.key().to_owned(), field_value);
    }
    values
}

fn seed_string(field_value: &mut FieldValue, field: &FormStringField) {
    let Some(default) = &field.default else {
        return;
    };
    let names_an_option = field
        .options
        .as_ref()
        .is_some_and(|options| options.iter().any(|option| option.value == *default));
    if names_an_option {
        field_value.selected = vec![default.clone()];
    } else {
        field_value.text.clone_from(default);
        field_value.custom = field
            .options
            .as_ref()
            .is_some_and(|options| !options.is_empty());
    }
}

/// The answer a single field contributes, or `None` when it has none yet
/// (`formCardState.ts:107-129`).
pub fn field_answer(field: &FormField, values: &FormValues) -> Option<FormValue> {
    let field_value = value_of(values, field.key());
    match field {
        FormField::String(field) => {
            let has_options = field
                .options
                .as_ref()
                .is_some_and(|options| !options.is_empty());
            if has_options && !field_value.custom {
                return field_value
                    .selected
                    .first()
                    .map(|picked| FormValue::Scalar(FormScalar::Text(picked.clone())));
            }
            let text = field_value.text.trim();
            (!text.is_empty()).then(|| FormValue::Scalar(FormScalar::Text(text.to_owned())))
        }
        FormField::Number(_) | FormField::Integer(_) => field_value
            .number
            .map(|number| FormValue::Scalar(FormScalar::Number(number))),
        FormField::Boolean(_) => Some(FormValue::Scalar(FormScalar::Bool(field_value.boolean))),
        FormField::Multiselect(_) => {
            let mut selected = field_value.selected.clone();
            let custom = field_value.text.trim();
            if field_value.custom && !custom.is_empty() {
                selected.push(custom.to_owned());
            }
            (!selected.is_empty()).then_some(FormValue::List(selected))
        }
        FormField::External(_) => None,
    }
}

mod evaluate;

pub use evaluate::{build_form_answer, evaluate_form, missing_required_keys, visible_fields};

#[cfg(test)]
#[path = "form_test.rs"]
mod form_test;
