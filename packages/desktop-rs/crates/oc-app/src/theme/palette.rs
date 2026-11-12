//! `requireTheme`'s defaults and `CSSVariableGenerator`'s mapping over `openchamber-dark`.
//!
//! Only the fields the window palette paints are resolved: the primary foreground, the subtle
//! surface and the focus ring are the ones `requireTheme` fills when the JSON leaves them out.
//! The rest of the mapping is `generateTailwindVariables`, `generateStatusColors` and
//! `generateSyntaxColors` in `packages/ui/src/lib/theme/cssGenerator.ts`.

use serde::Deserialize;

use super::{Color, Palette};

#[derive(Debug, Deserialize)]
struct ThemeDocument {
    colors: Colors,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Colors {
    primary: Primary,
    surface: Surface,
    interactive: Interactive,
    status: Status,
    syntax: Syntax,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Primary {
    base: Color,
    foreground: Option<Color>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Surface {
    background: Color,
    foreground: Color,
    muted: Color,
    muted_foreground: Color,
    elevated: Color,
    subtle: Option<Color>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Interactive {
    border: Color,
    focus_ring: Option<Color>,
}

#[derive(Debug, Deserialize)]
struct Status {
    error: Color,
    warning: Color,
    success: Color,
    info: Color,
}

#[derive(Debug, Deserialize)]
struct Syntax {
    base: SyntaxBase,
}

#[derive(Debug, Deserialize)]
struct SyntaxBase {
    keyword: Color,
    #[serde(rename = "type")]
    kind: Color,
    function: Color,
    number: Color,
    string: Color,
    operator: Color,
    variable: Color,
}

/// Parses the theme and resolves it into the window palette.
pub(super) fn parse(source: &str) -> Result<Palette, serde_json::Error> {
    let document: ThemeDocument = serde_json::from_str(source)?;
    Ok(document.colors.into_palette())
}

impl Colors {
    fn into_palette(self) -> Palette {
        let background = self.surface.background;
        let primary = self.primary.base;
        let subtle = self.surface.subtle.unwrap_or(self.surface.muted);
        let primary_foreground = self
            .primary
            .foreground
            .unwrap_or_else(|| on_color(primary, background));
        let ring = self
            .interactive
            .focus_ring
            .unwrap_or_else(|| with_opacity(primary, 0.5));
        Palette {
            background,
            foreground: self.surface.foreground,
            card: self.surface.elevated,
            popover: self.surface.elevated,
            primary,
            primary_foreground,
            secondary: self.surface.muted,
            muted: self.surface.muted,
            muted_foreground: self.surface.muted_foreground,
            accent: subtle,
            destructive: self.status.error,
            border: self.interactive.border,
            input: self.interactive.border,
            ring,
            sidebar: self.surface.muted,
            sidebar_accent: subtle,
            status_error: self.status.error,
            status_warning: self.status.warning,
            status_success: self.status.success,
            status_info: self.status.info,
            syntax_keyword: self.syntax.base.keyword,
            syntax_type: self.syntax.base.kind,
            syntax_function: self.syntax.base.function,
            syntax_number: self.syntax.base.number,
            syntax_string: self.syntax.base.string,
            syntax_operator: self.syntax.base.operator,
            syntax_variable: self.syntax.base.variable,
        }
    }
}

/// `withOpacity` from `packages/ui/src/lib/theme/color.ts`.
fn with_opacity(color: Color, alpha: f32) -> Color {
    let rgb = color.0 >> 8;
    let alpha = (alpha * 255.0).round() as u32;
    Color((rgb << 8) | (alpha & 0xff))
}

/// `onColor` from `packages/ui/src/lib/theme/color.ts`: black or white, whichever reads better.
fn on_color(background: Color, canvas: Color) -> Color {
    let black = contrast_ratio(Color(0x000000ff), background, canvas);
    let white = contrast_ratio(Color(0xffffffff), background, canvas);
    if black >= white {
        Color(0x000000ff)
    } else {
        Color(0xffffffff)
    }
}

/// `contrastRatio` from `color.ts`, over an opaque canvas.
fn contrast_ratio(foreground: Color, background: Color, canvas: Color) -> f64 {
    let surface = over(background, canvas);
    ratio(over(foreground, surface), surface)
}

/// Source-over composition of two colors, with an opaque result.
fn over(front: Color, back: Color) -> Color {
    let [front_r, front_g, front_b, front_a] = front.0.to_be_bytes();
    let [back_r, back_g, back_b, _] = back.0.to_be_bytes();
    let alpha = f64::from(front_a) / 255.0;
    let channel = |front: u8, back: u8| {
        (f64::from(front) * alpha + f64::from(back) * (1.0 - alpha)).round() as u8
    };
    Color(u32::from_be_bytes([
        channel(front_r, back_r),
        channel(front_g, back_g),
        channel(front_b, back_b),
        0xff,
    ]))
}

fn ratio(first: Color, second: Color) -> f64 {
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn luminance(color: Color) -> f64 {
    let [red, green, blue, _] = color.0.to_be_bytes();
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}

#[cfg(test)]
#[path = "palette_test.rs"]
mod palette_test;
