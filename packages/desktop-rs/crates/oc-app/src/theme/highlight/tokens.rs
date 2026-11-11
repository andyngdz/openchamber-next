//! `resolveSyntaxTokens` from `packages/ui/src/lib/theme/syntax.ts`.

use std::collections::BTreeMap;

use super::names::Token;

fn base_of(base: &BTreeMap<String, String>, name: Token) -> Option<String> {
    base.get(name.as_str()).cloned()
}

fn raw_of(raw: &BTreeMap<String, String>, name: Token) -> Option<String> {
    raw.get(name.as_str()).cloned()
}

/// Adds the inherited color unless the theme states its own.
pub(super) fn inherit(base: &mut BTreeMap<String, String>, name: Token, color: &str) {
    base.entry(name.as_str().to_owned())
        .or_insert_with(|| color.to_owned());
}

/// The resolved token map: the theme's own tokens override every computed relationship.
pub(super) fn resolve_tokens(
    base: &BTreeMap<String, String>,
    raw: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut tokens = inherited(base);
    tokens.extend(derived(base, raw));
    tokens.extend(
        raw.iter()
            .map(|(name, color)| (name.clone(), color.clone())),
    );
    tokens
}

/// Relationships that read only the base colors.
fn inherited(base: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    [
        (Token::CommentDoc, base_of(base, Token::Comment)),
        (Token::StringEscape, base_of(base, Token::Foreground)),
        (Token::StringInterpolation, base_of(base, Token::Variable)),
        (Token::StringRegex, base_of(base, Token::String)),
        (Token::KeywordControl, base_of(base, Token::Keyword)),
        (Token::KeywordOperator, base_of(base, Token::Operator)),
        (Token::KeywordImport, base_of(base, Token::Operator)),
        (Token::KeywordReturn, base_of(base, Token::Keyword)),
        (Token::StorageModifier, base_of(base, Token::Keyword)),
        (Token::FunctionCall, base_of(base, Token::Function)),
        (Token::FunctionBuiltin, base_of(base, Token::Function)),
        (Token::Method, base_of(base, Token::Function)),
        (Token::VariableBuiltin, base_of(base, Token::Variable)),
        (Token::VariableReadonly, base_of(base, Token::Number)),
        (Token::VariableOther, base_of(base, Token::Variable)),
        (Token::VariableGlobal, base_of(base, Token::String)),
        (Token::VariableLocal, base_of(base, Token::Variable)),
        (Token::TypePrimitive, base_of(base, Token::Type)),
        (Token::Interface, base_of(base, Token::Type)),
        (Token::TypeParameter, base_of(base, Token::Type)),
        (Token::Boolean, base_of(base, Token::Keyword)),
        (Token::Null, base_of(base, Token::Number)),
        (Token::Constant, base_of(base, Token::Number)),
        (Token::Punctuation, base_of(base, Token::Comment)),
        (Token::Bracket, base_of(base, Token::Foreground)),
        (Token::Tag, base_of(base, Token::Keyword)),
        (Token::TagAttributeValue, base_of(base, Token::String)),
        (Token::Decorator, base_of(base, Token::Function)),
        (Token::Annotation, base_of(base, Token::Function)),
        (Token::Namespace, base_of(base, Token::Type)),
        (Token::Module, base_of(base, Token::Operator)),
        (Token::Label, base_of(base, Token::Operator)),
        (Token::Macro, base_of(base, Token::Keyword)),
        (Token::Preprocessor, base_of(base, Token::Operator)),
        (Token::Regex, base_of(base, Token::String)),
        (Token::Url, base_of(base, Token::String)),
        (Token::Exception, base_of(base, Token::Operator)),
    ]
    .into_iter()
    .filter_map(|(name, color)| color.map(|color| (name.as_str().to_owned(), color)))
    .collect()
}

/// Relationships that read the theme's own tokens.
fn derived(
    base: &BTreeMap<String, String>,
    raw: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let property = raw_of(raw, Token::VariableProperty).or_else(|| base_of(base, Token::Variable));
    let class_name = raw_of(raw, Token::ClassName)
        .or_else(|| raw_of(raw, Token::Class))
        .or_else(|| base_of(base, Token::Type));
    let comment = base_of(base, Token::Comment);
    [
        (
            Token::MethodCall,
            raw_of(raw, Token::Method).or_else(|| base_of(base, Token::Function)),
        ),
        (Token::VariableProperty, property.clone()),
        (
            Token::Parameter,
            raw_of(raw, Token::StringEscape).or_else(|| base_of(base, Token::Foreground)),
        ),
        (
            Token::TypeInterface,
            raw_of(raw, Token::Interface).or_else(|| base_of(base, Token::Type)),
        ),
        (Token::Class, class_name.clone()),
        (Token::ClassName, class_name.clone()),
        (Token::Struct, class_name.clone()),
        (Token::Enum, class_name),
        (
            Token::Delimiter,
            raw_of(raw, Token::Punctuation).or_else(|| comment.clone()),
        ),
        (
            Token::JsxTag,
            raw_of(raw, Token::Tag).or_else(|| base_of(base, Token::Keyword)),
        ),
        (Token::TagAttribute, property.clone()),
        (
            Token::TagBracket,
            raw_of(raw, Token::Punctuation).or_else(|| comment.clone()),
        ),
        (Token::Key, property),
    ]
    .into_iter()
    .filter_map(|(name, color)| color.map(|color| (name.as_str().to_owned(), color)))
    .collect()
}

#[cfg(test)]
#[path = "tokens_test.rs"]
mod tokens_test;
