//! The markdown a recovered question's answer is sent as (`formSerializers.ts:63-79`).

use oc_client::form::{FormAnswer, FormScalar, FormValue};
use oc_client::form_field::FormField;
use serde::Serialize;
use serde::ser::SerializeMap;
use serde_json::{Number, Value};

/// New-turn context includes only answered fields and preserves typed values.
pub fn serialize_form_answer_as_markdown(
    form: &oc_client::form::FormInfo,
    answer: &FormAnswer,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let title = form.title.trim();
    if !title.is_empty() {
        lines.push(format!("# {title}"));
        lines.push(String::new());
    }
    for field in &form.fields {
        let Some(value) = answer.get(field.key()) else {
            continue;
        };
        let heading = field_title(field);
        lines.push(format!("## {heading}"));
        lines.push(String::new());
        if let Some(description) = field_description(field)
            && !description.is_empty()
        {
            lines.push(description.to_owned());
            lines.push(String::new());
        }
        lines.push(format!("**Answer:** {}", answer_text(value)));
        lines.push(String::new());
    }
    lines.push("```json".to_owned());
    lines.push(ordered_json(form, answer));
    lines.push("```".to_owned());
    lines.join("\n").trim_end().to_owned()
}

fn field_title(field: &FormField) -> String {
    match field {
        FormField::String(field) => title_or_key(field.title.as_deref(), &field.key),
        FormField::Number(field) => title_or_key(field.title.as_deref(), &field.key),
        FormField::Integer(field) => title_or_key(field.title.as_deref(), &field.key),
        FormField::Boolean(field) => title_or_key(field.title.as_deref(), &field.key),
        FormField::Multiselect(field) => title_or_key(field.title.as_deref(), &field.key),
        FormField::External(field) => title_or_key(field.title.as_deref(), &field.key),
    }
}

fn title_or_key(title: Option<&str>, key: &str) -> String {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(key)
        .to_owned()
}

fn field_description(field: &FormField) -> Option<&str> {
    match field {
        FormField::String(field) => field.description.as_deref(),
        FormField::Number(field) => field.description.as_deref(),
        FormField::Integer(field) => field.description.as_deref(),
        FormField::Boolean(field) => field.description.as_deref(),
        FormField::Multiselect(field) => field.description.as_deref(),
        FormField::External(field) => field.description.as_deref(),
    }
}

fn answer_text(value: &FormValue) -> String {
    match value {
        FormValue::List(entries) => entries.join(", "),
        FormValue::Scalar(FormScalar::Text(text)) => text.clone(),
        FormValue::Scalar(FormScalar::Number(number)) => number.to_string(),
        FormValue::Scalar(FormScalar::Bool(value)) => value.to_string(),
    }
}

/// A JSON object in field declaration order, the way `JSON.stringify` writes React's answer.
struct OrderedAnswer<'a> {
    fields: &'a [FormField],
    answer: &'a FormAnswer,
}

impl Serialize for OrderedAnswer<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        for field in self.fields {
            if let Some(value) = self.answer.get(field.key()) {
                map.serialize_entry(field.key(), &json_value(value))?;
            }
        }
        map.end()
    }
}

fn json_value(value: &FormValue) -> Value {
    match value {
        FormValue::List(entries) => {
            Value::Array(entries.iter().cloned().map(Value::String).collect())
        }
        FormValue::Scalar(FormScalar::Text(text)) => Value::String(text.clone()),
        FormValue::Scalar(FormScalar::Number(number)) => number_json(*number),
        FormValue::Scalar(FormScalar::Bool(value)) => Value::Bool(*value),
    }
}

fn number_json(number: f64) -> Value {
    if number.fract() == 0.0 && number >= i64::MIN as f64 && number <= i64::MAX as f64 {
        return Value::Number(Number::from(number as i64));
    }
    Number::from_f64(number).map_or(Value::Null, Value::Number)
}

fn ordered_json(form: &oc_client::form::FormInfo, answer: &FormAnswer) -> String {
    serde_json::to_string(&OrderedAnswer {
        fields: &form.fields,
        answer,
    })
    .unwrap_or_else(|_| "{}".to_owned())
}

#[cfg(test)]
#[path = "form_serializers_test.rs"]
mod form_serializers_test;
