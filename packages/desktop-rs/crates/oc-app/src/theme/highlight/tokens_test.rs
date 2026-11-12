use std::collections::BTreeMap;

use super::super::names::Token;
use super::resolve_tokens;

fn colors(pairs: &[(Token, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, color)| (name.as_str().to_owned(), (*color).to_owned()))
        .collect()
}

#[test]
fn a_theme_token_overrides_its_relationship() {
    let base = colors(&[(Token::Variable, "#111111"), (Token::Type, "#333333")]);
    let raw = colors(&[(Token::VariableProperty, "#222222")]);

    let tokens = resolve_tokens(&base, &raw);

    assert_eq!(
        tokens.get("variableProperty").map(String::as_str),
        Some("#222222")
    );
    assert_eq!(
        tokens.get("tagAttribute").map(String::as_str),
        Some("#222222")
    );
    assert_eq!(tokens.get("key").map(String::as_str), Some("#222222"));
}

#[test]
fn property_falls_back_to_the_base_variable() {
    let base = colors(&[(Token::Variable, "#111111")]);
    let raw = BTreeMap::new();

    let tokens = resolve_tokens(&base, &raw);

    assert_eq!(
        tokens.get("variableProperty").map(String::as_str),
        Some("#111111")
    );
    assert_eq!(
        tokens.get("tagAttribute").map(String::as_str),
        Some("#111111")
    );
}

#[test]
fn a_class_name_reaches_the_class_roles() {
    let base = colors(&[(Token::Type, "#333333")]);
    let raw = colors(&[(Token::ClassName, "#444444")]);

    let tokens = resolve_tokens(&base, &raw);

    for role in ["class", "className", "struct", "enum"] {
        assert_eq!(
            tokens.get(role).map(String::as_str),
            Some("#444444"),
            "{role}"
        );
    }
}

#[test]
fn the_class_name_falls_back_to_the_base_type() {
    let base = colors(&[(Token::Type, "#333333")]);
    let raw = BTreeMap::new();

    let tokens = resolve_tokens(&base, &raw);

    for role in ["class", "className", "struct", "enum"] {
        assert_eq!(
            tokens.get(role).map(String::as_str),
            Some("#333333"),
            "{role}"
        );
    }
}

#[test]
fn every_theme_token_is_kept_even_without_a_relationship() {
    let base = BTreeMap::new();
    let raw = colors(&[(Token::Module, "#555555")]);

    let tokens = resolve_tokens(&base, &raw);

    assert_eq!(tokens.get("module").map(String::as_str), Some("#555555"));
}
