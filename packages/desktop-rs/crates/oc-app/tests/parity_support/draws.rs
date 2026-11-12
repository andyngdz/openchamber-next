//! What each control draws, compared with the reference: its icons (name and color), its
//! marks (color) and its text runs (text and color), in order, colors within 1 per channel.
//!
//! Only the regions in [`DRAWN_REGIONS`] are compared. A unit that makes a region match adds
//! it, so every commit stays green, and the last surface unit removes the list (design.md,
//! "Rollout of the comparison"). The frame check covers every region from the start.

use std::collections::HashMap;

use serde_json::Value;

/// The regions whose draws already match the reference in every state.
pub const DRAWN_REGIONS: &[&str] = &[];

/// One channel may differ by this much.
const CHANNEL_TOLERANCE: u8 = 1;

/// The three draw lists a control records.
#[derive(Debug, Clone, Copy)]
enum DrawKind {
    Icons,
    Marks,
    Texts,
}

impl DrawKind {
    const ALL: [Self; 3] = [Self::Icons, Self::Marks, Self::Texts];

    /// The list's field in the dump and the reference.
    fn field(self) -> &'static str {
        match self {
            Self::Icons => "icons",
            Self::Marks => "marks",
            Self::Texts => "texts",
        }
    }

    /// The identity of one draw besides its color: an icon's name, a text run's text.
    fn label(self, draw: &Value) -> Value {
        match self {
            Self::Icons => draw["name"].clone(),
            Self::Texts => draw["text"].clone(),
            Self::Marks => Value::Null,
        }
    }
}

/// A color as the capture and the dump write it, `#rrggbbaa`.
fn channels(color: &str) -> Option<[u8; 4]> {
    let digits = color.strip_prefix('#').filter(|digits| digits.len() == 8)?;
    let mut parsed = [0; 4];
    for (index, slot) in parsed.iter_mut().enumerate() {
        *slot = u8::from_str_radix(digits.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(parsed)
}

/// Two recorded colors match when both are absent, or both parse and differ by at most one
/// per channel.
fn colors_match(expected: &Value, drawn: &Value) -> bool {
    match (expected.as_str(), drawn.as_str()) {
        (None, None) => true,
        (Some(expected), Some(drawn)) => match (channels(expected), channels(drawn)) {
            (Some(expected), Some(drawn)) => expected
                .iter()
                .zip(drawn)
                .all(|(wanted, got)| wanted.abs_diff(got) <= CHANNEL_TOLERANCE),
            _ => false,
        },
        _ => false,
    }
}

/// One draw list in the shape a failure message shows.
fn summary(kind: DrawKind, draws: &[Value]) -> Vec<String> {
    draws
        .iter()
        .map(|draw| format!("{} {}", kind.label(draw), draw["color"]))
        .collect()
}

fn draw_list(control: &Value, kind: DrawKind) -> &[Value] {
    control[kind.field()].as_array().map_or(&[], Vec::as_slice)
}

/// Whether two controls draw the same list of this kind.
fn same_draws(kind: DrawKind, expected: &[Value], drawn: &[Value]) -> bool {
    expected.len() == drawn.len()
        && expected.iter().zip(drawn).all(|(wanted, got)| {
            kind.label(wanted) == kind.label(got) && colors_match(&wanted["color"], &got["color"])
        })
}

/// The dump's controls of a region by key, repeated keys in order.
fn dumped_by_key<'a>(layout: &'a Value, region: &str) -> HashMap<&'a str, Vec<&'a Value>> {
    let mut by_key: HashMap<&str, Vec<&Value>> = HashMap::new();
    let controls = layout["regions"][region]["controls"].as_array();
    for control in controls.into_iter().flatten() {
        if let Some(key) = control["key"].as_str() {
            by_key.entry(key).or_default().push(control);
        }
    }
    by_key
}

/// One reference control against the dump's control of the same key.
fn control_draw_differences(key: &str, control: &Value, drawn: &Value) -> Vec<String> {
    DrawKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let (want, got) = (draw_list(control, kind), draw_list(drawn, kind));
            (!same_draws(kind, want, got)).then(|| {
                format!(
                    "{key} {}: expected {:?}, drawn {:?}",
                    kind.field(),
                    summary(kind, want),
                    summary(kind, got)
                )
            })
        })
        .collect()
}

/// Every shown reference control in `regions` whose draws differ from the dump's control of
/// the same key. A control missing from the dump is left to the key comparison.
pub fn draw_differences(expected: &Value, actual: &Value, regions: &[&str]) -> Vec<String> {
    let mut differences = Vec::new();
    for &region in regions {
        let mut dumped = dumped_by_key(actual, region);
        let wanted = expected["regions"][region]["controls"].as_array();
        for control in wanted.into_iter().flatten() {
            let Some(key) = control["key"].as_str() else {
                continue;
            };
            let Some(same_key) = dumped.get_mut(key).filter(|list| !list.is_empty()) else {
                continue;
            };
            let drawn = same_key.remove(0);
            if control["shown"] == true {
                differences.extend(control_draw_differences(key, control, drawn));
            }
        }
    }
    differences
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::draw_differences;

    fn layout(color: &str, name: &str) -> serde_json::Value {
        json!({"regions": {"header": {"controls": [{
            "key": "header/Menu", "shown": true,
            "icons": [{"name": name, "color": color}], "marks": [], "texts": []
        }]}}})
    }

    #[test]
    fn a_color_one_step_off_still_matches() {
        let differences = draw_differences(
            &layout("#10203040", "menu"),
            &layout("#11213141", "menu"),
            &["header"],
        );
        assert!(differences.is_empty(), "{differences:?}");
    }

    #[test]
    fn a_wrong_color_or_icon_names_the_control_and_both_values() {
        for drawn in [layout("#12203040", "menu"), layout("#10203040", "close")] {
            let differences = draw_differences(&layout("#10203040", "menu"), &drawn, &["header"]);
            assert_eq!(differences.len(), 1);
            assert!(differences[0].starts_with("header/Menu icons: expected"));
        }
    }

    #[test]
    fn regions_outside_the_list_are_not_compared() {
        let differences =
            draw_differences(&layout("#10203040", "menu"), &layout("#ffffffff", "x"), &[]);
        assert!(differences.is_empty());
    }
}
