//! The display strings React's model picker and composer build.
//!
//! Ports `getModelDisplayName` (`packages/ui/src/lib/modelDisplay.ts:21-31,234-250`) and
//! `formatCompactNumber` (`packages/ui/src/lib/numberFormat.ts:4-9`), plus the effort label
//! from `mobileControlsUtils.ts:89-98`.

/// `getModelDisplayName`'s default maximum (`ModelPickerList.tsx:39`).
const MODEL_NAME_MAX: usize = 40;

/// A model's display name: its trimmed server name, else its id, cut to `max`.
pub(crate) fn model_display_name(name: &str, id: &str) -> String {
    let trimmed = name.trim();
    let source = if trimmed.is_empty() {
        id.trim()
    } else {
        trimmed
    };
    truncate(source, MODEL_NAME_MAX)
}

/// `truncate` in `modelDisplay.ts:21-31`: keep `max` UTF-16 units, ending with `...`.
pub(crate) fn truncate(value: &str, max: usize) -> String {
    if max == 0 || value.encode_utf16().count() <= max {
        return value.to_owned();
    }
    if max <= 3 {
        return value.chars().take(max).collect();
    }
    let mut out = String::new();
    let mut units = 0;
    for character in value.chars() {
        let width = character.len_utf16();
        if units + width > max - 3 {
            break;
        }
        units += width;
        out.push(character);
    }
    out.push_str("...");
    out
}

/// `formatCompactNumber`: short compact notation, at most one fraction digit.
pub(crate) fn compact_tokens(value: u64) -> String {
    let (scaled, suffix) = if value >= 1_000_000_000 {
        (value as f64 / 1_000_000_000., "B")
    } else if value >= 1_000_000 {
        (value as f64 / 1_000_000., "M")
    } else if value >= 1_000 {
        (value as f64 / 1_000., "K")
    } else {
        return value.to_string();
    };
    let rounded = (scaled * 10.).round() / 10.;
    let text = format!("{rounded:.1}");
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("{text}{suffix}")
}

/// `formatEffortLabel`: Default for no level, otherwise the level capitalized.
pub(crate) fn effort_label(variant: Option<&str>) -> String {
    match variant.map(str::trim).filter(|level| !level.is_empty()) {
        None => "Default".to_owned(),
        Some(level) if level.chars().all(|c| c.is_ascii_digit() || c == '.') => level.to_owned(),
        Some(level) => {
            let mut chars = level.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        }
    }
}

/// The agent name React shows: the server's display name, else the id capitalized.
pub(crate) fn agent_display_name(display_name: &str, id: &str) -> String {
    let name = display_name.trim();
    if !name.is_empty() {
        return name.to_owned();
    }
    effort_label(Some(id))
}

#[cfg(test)]
#[path = "model_names_test.rs"]
mod model_names_test;
