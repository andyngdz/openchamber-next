use std::collections::BTreeMap;

use super::super::names::Token;
use super::{Colors, DIFF_ADDED, Metadata, Resolved, Status, Surface, Syntax, ThemeDocument};

fn colors(pairs: &[(Token, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, color)| (name.as_str().to_owned(), (*color).to_owned()))
        .collect()
}

// Enough of a theme for these checks: every base role, one token override, one diff color,
// and the status fallback for the other diff role.
fn theme_document() -> ThemeDocument {
    ThemeDocument {
        metadata: Metadata {
            id: "test-theme".to_owned(),
            variant: "dark".to_owned(),
        },
        colors: Colors {
            surface: Surface {
                background: "#000000".to_owned(),
                foreground: "#ffffff".to_owned(),
            },
            status: Status {
                error: "#ff0000".to_owned(),
                success: "#00ff00".to_owned(),
            },
            syntax: Syntax {
                base: colors(&[
                    (Token::Comment, "#111111"),
                    (Token::Keyword, "#222222"),
                    (Token::String, "#333333"),
                    (Token::Number, "#444444"),
                    (Token::Function, "#555555"),
                    (Token::Variable, "#666666"),
                    (Token::Type, "#777777"),
                    (Token::Operator, "#888888"),
                ]),
                tokens: colors(&[(Token::VariableProperty, "#999999")]),
                highlights: BTreeMap::from([(DIFF_ADDED.to_owned(), "#aaaaaa".to_owned())]),
            },
        },
    }
}

fn resolved() -> Resolved {
    theme_document().resolve()
}

#[test]
fn a_keyword_paints_with_the_base_keyword() {
    let style = resolved().style("keyword").expect("keyword has a rule");

    assert_eq!(style.color.as_deref(), Some("#222222"));
    assert!(!style.bold && !style.italic);
}

#[test]
fn a_dotted_name_falls_back_to_its_first_segment() {
    let theme = resolved();

    assert_eq!(theme.style("keyword.modifier"), theme.style("keyword"));
    assert_eq!(
        theme.style("punctuation.bracket.rust"),
        theme.style("punctuation")
    );
}

#[test]
fn the_diff_roles_read_the_highlights_and_fall_back_to_status() {
    let theme = resolved();

    assert_eq!(
        theme.style("diff.plus").and_then(|style| style.color),
        Some("#aaaaaa".to_owned())
    );
    assert_eq!(
        theme.style("diff.minus").and_then(|style| style.color),
        Some("#ff0000".to_owned())
    );
}

#[test]
fn emphasis_carries_type_attributes_without_a_color() {
    let theme = resolved();
    let emphasis = theme.style("emphasis").expect("emphasis has a rule");
    let strong = theme.style("emphasis.strong").expect("strong has a rule");

    assert_eq!(emphasis.color, None);
    assert!(emphasis.italic && !emphasis.bold);
    assert_eq!(strong.color, None);
    assert!(strong.bold && !strong.italic);
}

#[test]
fn the_base_background_is_inherited_from_the_surface() {
    let theme = resolved();

    assert_eq!(
        theme.base_color(Token::Background),
        Some("#000000".to_owned())
    );
}

#[test]
fn every_rule_resolves() {
    assert_eq!(resolved().styles().count(), 66);
}
