//! The syntax colors of the Electron default dark theme as a GPUI Kit highlighter theme.
//!
//! A port of `packages/ui/src/lib/shiki/zedThemeFromAppTheme.ts` over `resolveSyntaxTokens`
//! from `packages/ui/src/lib/theme/syntax.ts`: the token table, the dotted-name parent
//! fallback, the bold and italic roles, and the diff roles. `openchamber-dark.json` is the
//! Electron default dark theme (`DEFAULT_DARK_THEME_ID`); its copy under `assets/themes` is
//! embedded here, and a test keeps the copy byte for byte equal to the source.

mod names;
mod rules;
mod syntax;
mod tokens;

use std::sync::{Arc, OnceLock};

use gpui_kit::component::highlighter::HighlightTheme;
use serde_json::{Map, Value, json};

use crate::logging::APP;
use names::Token;
use syntax::{ThemeDocument, TokenStyle};

/// The font style name `buildSyntax` writes for an italic role.
const ITALIC: &str = "italic";

/// The Electron default dark theme, copied from `packages/ui/src/lib/theme/themes/`.
const DARK_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/openchamber-dark.json"
));

/// The dark theme's highlighter, parsed once from the embedded JSON.
pub(crate) fn dark() -> Arc<HighlightTheme> {
    static DARK: OnceLock<Arc<HighlightTheme>> = OnceLock::new();
    DARK.get_or_init(|| match parse(DARK_THEME) {
        Ok(theme) => Arc::new(theme),
        Err(error) => {
            tracing::warn!(
                target: APP,
                %error,
                "[APP] Embedded highlight theme did not parse"
            );
            HighlightTheme::default_dark()
        }
    })
    .clone()
}

fn parse(source: &str) -> Result<HighlightTheme, serde_json::Error> {
    let document: ThemeDocument = serde_json::from_str(source)?;
    theme_from(&document)
}

fn theme_from(document: &ThemeDocument) -> Result<HighlightTheme, serde_json::Error> {
    let resolved = document.resolve();
    let mut syntax = Map::new();
    for (name, style) in resolved.styles() {
        syntax.insert(name.as_str().to_owned(), settings(&style));
    }
    let mut style = Map::new();
    if let Some(background) = resolved.base_color(Token::Background) {
        style.insert("editor.background".to_owned(), Value::String(background));
    }
    if let Some(foreground) = resolved.base_color(Token::Foreground) {
        style.insert("editor.foreground".to_owned(), Value::String(foreground));
    }
    style.insert("syntax".to_owned(), Value::Object(syntax));
    let theme_value = json!({
        "name": document.metadata.id,
        "appearance": document.metadata.variant,
        "style": style,
    });
    serde_json::from_value(theme_value)
}

fn settings(style: &TokenStyle) -> Value {
    let mut settings = Map::new();
    if let Some(color) = &style.color {
        settings.insert("color".to_owned(), Value::String(color.clone()));
    }
    if style.bold {
        settings.insert("font_weight".to_owned(), Value::from(700));
    }
    if style.italic {
        settings.insert("font_style".to_owned(), Value::String(ITALIC.to_owned()));
    }
    Value::Object(settings)
}

#[cfg(test)]
#[path = "highlight_test.rs"]
mod highlight_test;
