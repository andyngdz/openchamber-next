//! The app theme JSON this port reads, and the resolved syntax it yields.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::names::Token;
use super::rules::{RULES, Role};
use super::tokens::{inherit, resolve_tokens};

/// The `colors.syntax.highlights` key for added lines.
const DIFF_ADDED: &str = "diffAdded";
/// The `colors.syntax.highlights` key for removed lines.
const DIFF_REMOVED: &str = "diffRemoved";

/// The parts of an app theme this port reads.
#[derive(Debug, Deserialize)]
pub(super) struct ThemeDocument {
    pub(super) metadata: Metadata,
    pub(super) colors: Colors,
}

/// The theme's identity, which names the highlighter theme and its appearance.
#[derive(Debug, Deserialize)]
pub(super) struct Metadata {
    pub(super) id: String,
    pub(super) variant: String,
}

/// The theme colors this port reads, from `colors` in the app theme JSON.
#[derive(Debug, Deserialize)]
pub(super) struct Colors {
    surface: Surface,
    status: Status,
    syntax: Syntax,
}

#[derive(Debug, Deserialize)]
struct Surface {
    background: String,
    foreground: String,
}

#[derive(Debug, Deserialize)]
struct Status {
    error: String,
    success: String,
}

#[derive(Debug, Deserialize)]
struct Syntax {
    #[serde(default)]
    base: BTreeMap<String, String>,
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    #[serde(default)]
    highlights: BTreeMap<String, String>,
}

/// One token name's color and type attributes, as `buildSyntax` builds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TokenStyle {
    pub(super) color: Option<String>,
    pub(super) bold: bool,
    pub(super) italic: bool,
}

/// One theme's syntax with every relationship resolved, as `buildZedThemeFromAppTheme` sees
/// it: `base` has its inherited background and foreground, `tokens` is the full
/// `resolveSyntaxTokens` result, and the diff roles fall back to the status colors.
pub(super) struct Resolved {
    base: BTreeMap<String, String>,
    tokens: BTreeMap<String, String>,
    inserted: String,
    deleted: String,
}

impl ThemeDocument {
    /// Resolves the theme the way the React pipeline (`requireTheme` then
    /// `buildZedThemeFromAppTheme`) resolves it.
    pub(super) fn resolve(&self) -> Resolved {
        let base = self.base_colors();
        let tokens = resolve_tokens(&base, &self.colors.syntax.tokens);
        Resolved {
            base,
            tokens,
            inserted: self.highlight(DIFF_ADDED, &self.colors.status.success),
            deleted: self.highlight(DIFF_REMOVED, &self.colors.status.error),
        }
    }

    fn base_colors(&self) -> BTreeMap<String, String> {
        let mut base = self.colors.syntax.base.clone();
        inherit(
            &mut base,
            Token::Background,
            &self.colors.surface.background,
        );
        inherit(
            &mut base,
            Token::Foreground,
            &self.colors.surface.foreground,
        );
        base
    }

    fn highlight(&self, name: &str, fallback: &str) -> String {
        self.colors
            .syntax
            .highlights
            .get(name)
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    }
}

impl Resolved {
    /// The base color `name` states, if the theme has one.
    pub(super) fn base_color(&self, name: Token) -> Option<String> {
        self.base.get(name.as_str()).cloned()
    }

    fn token_color(&self, name: Token) -> Option<String> {
        self.tokens.get(name.as_str()).cloned()
    }

    /// The style for `name`, falling back to its first dot-separated segment the way GPUI
    /// Kit's `SyntaxColors::style` does.
    pub(super) fn style(&self, name: &str) -> Option<TokenStyle> {
        let rule = rule(name)?;
        let color = match rule.role {
            None => None,
            Some(Role::Base(key)) => self.base_color(key),
            Some(Role::Token(key)) => self.token_color(key),
            Some(Role::Inserted) => Some(self.inserted.clone()),
            Some(Role::Deleted) => Some(self.deleted.clone()),
        };
        Some(TokenStyle {
            color,
            bold: rule.bold,
            italic: rule.italic,
        })
    }

    /// Every `TOKEN_RULES` entry with its resolved style.
    pub(super) fn styles(&self) -> impl Iterator<Item = (Token, TokenStyle)> + '_ {
        RULES.iter().filter_map(|rule| {
            self.style(rule.name.as_str())
                .map(|style| (rule.name, style))
        })
    }
}

fn rule(name: &str) -> Option<&'static super::rules::Rule> {
    RULES
        .iter()
        .find(|rule| rule.name.as_str() == name)
        .or_else(|| {
            name.split_once('.')
                .and_then(|(parent, _)| RULES.iter().find(|rule| rule.name.as_str() == parent))
        })
}

#[cfg(test)]
#[path = "syntax_test.rs"]
mod syntax_test;
