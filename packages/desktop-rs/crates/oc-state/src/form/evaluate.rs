//! `when` and `hidden` evaluation over earlier active answers (`formCardState.ts:139-209`).

use oc_client::form::{FormAnswer, FormScalar, FormValue, FormWhen, FormWhenOp};
use oc_client::form_field::FormField;

use super::{FormEvaluation, field_answer, value_of};

/// Mirrors OpenCode's `matches()`: a clause against an unanswered field is false for both
/// `eq` and `neq`.
fn clause_matches(clause: &FormWhen, answer: Option<&FormValue>) -> bool {
    let Some(answer) = answer else {
        return false;
    };
    let hit = match answer {
        FormValue::List(entries) => match &clause.value {
            FormScalar::Text(text) => entries.iter().any(|entry| entry == text),
            FormScalar::Number(_) | FormScalar::Bool(_) => false,
        },
        FormValue::Scalar(scalar) => scalar == &clause.value,
    };
    match clause.op {
        FormWhenOp::Eq => hit,
        FormWhenOp::Neq => !hit,
    }
}

/// Walks the fields in declaration order, every `when` clause reading only earlier active
/// answers (`formCardState.ts:158-174`).
pub fn evaluate_form(fields: &[FormField], values: &crate::form::FormValues) -> FormEvaluation {
    let mut active = Vec::new();
    let mut answer = FormAnswer::new();
    for field in fields {
        let matches = match field {
            FormField::String(field) => clauses_match(&field.when, &answer),
            FormField::Number(field) => clauses_match(&field.when, &answer),
            FormField::Integer(field) => clauses_match(&field.when, &answer),
            FormField::Boolean(field) => clauses_match(&field.when, &answer),
            FormField::Multiselect(field) => clauses_match(&field.when, &answer),
            FormField::External(_) => true,
        };
        if !matches {
            continue;
        }
        active.push(field.clone());
        if let FormField::External(field) = field {
            if value_of(values, &field.key).acknowledged {
                answer.insert(field.key.clone(), FormValue::Scalar(FormScalar::Bool(true)));
            }
            continue;
        }
        if let Some(answer_value) = field_answer(field, values) {
            answer.insert(field.key().to_owned(), answer_value);
        }
    }
    FormEvaluation { active, answer }
}

fn clauses_match(clauses: &[FormWhen], answer: &FormAnswer) -> bool {
    clauses
        .iter()
        .all(|clause| clause_matches(clause, answer.get(&clause.key)))
}

/// Fields the dock renders right now, in declaration order.
pub fn visible_fields(fields: &[FormField], values: &crate::form::FormValues) -> Vec<FormField> {
    evaluate_form(fields, values).active
}

/// Keys of visible fields the server would refuse the reply without
/// (`formCardState.ts:185-200`).
pub fn missing_required_keys(
    fields: &[FormField],
    values: &crate::form::FormValues,
) -> Vec<String> {
    let FormEvaluation { active, answer } = evaluate_form(fields, values);
    let mut missing = Vec::new();
    for field in &active {
        let answer_value = answer.get(field.key());
        if matches!(field, FormField::External(_)) {
            if answer_value != Some(&FormValue::Scalar(FormScalar::Bool(true))) {
                missing.push(field.key().to_owned());
            }
            continue;
        }
        if !is_required(field) {
            continue;
        }
        if answer_value
            .is_none_or(|value| matches!(value, FormValue::List(list) if list.is_empty()))
        {
            missing.push(field.key().to_owned());
        }
    }
    missing
}

fn is_required(field: &FormField) -> bool {
    match field {
        FormField::String(field) => field.required,
        FormField::Number(field) => field.required,
        FormField::Integer(field) => field.required,
        FormField::Boolean(field) => field.required,
        FormField::Multiselect(field) => field.required,
        FormField::External(_) => false,
    }
}

/// The reply payload: only active fields contribute (`formCardState.ts:207-209`).
pub fn build_form_answer(fields: &[FormField], values: &crate::form::FormValues) -> FormAnswer {
    evaluate_form(fields, values).answer
}

#[cfg(test)]
#[path = "evaluate_test.rs"]
mod evaluate_test;
