//! How the parity test compares a region's controls with the reference: exact keys in order,
//! and the chat column as one contiguous window of the dump's controls. React virtualizes the
//! message list, so the reference holds only the messages mounted around the viewport; the
//! dump holds every message. Earlier and later messages' chrome may therefore sit outside the
//! reference's window, while a control missing from the window itself still fails.

use std::collections::HashSet;

use serde_json::Value;

pub const REGIONS: [&str; 7] = [
    "sidebar",
    "header",
    "chatColumn",
    "composer",
    "workStatusPanel",
    "iconRail",
    "popover",
];

mod chat_window;

use chat_window::chat_column_difference;

/// The key and role of each control of a region, in order.
fn controls(layout: &Value, region: &str) -> Vec<(String, String)> {
    layout["regions"][region]["controls"]
        .as_array()
        .map(|controls| {
            controls
                .iter()
                .filter_map(|control| {
                    let key = control["key"].as_str()?.to_owned();
                    let role = control["role"].as_str().unwrap_or_default().to_owned();
                    Some((key, role))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The key without its `#n`, and whether it had one.
fn base_key(key: &str) -> (&str, bool) {
    match key.rsplit_once('#') {
        Some((base, ordinal)) if ordinal.parse::<usize>().is_ok() => (base, true),
        _ => (key, false),
    }
}

/// Base keys of the tool rows the spike titles: `glob` is the only tool in the fixtures and
/// its title is "Find Files <pattern>" (`views/turns.rs` `ToolTitle`).
const TOOL_ROW_BASES: [&str; 1] = ["chatColumn/Find Files "];

/// A chat-column tool row: a button whose key has `#n` and whose base is a tool title.
///
/// These are the only keys virtualization renumbers. React virtualizes the timeline
/// (`packages/ui/src/components/chat/MessageList.tsx`, `@legendapp/list`), so the reference
/// lists only the rows mounted around the viewport and counts `#n` from the first mounted
/// one, while the spike draws every row. Every other `#n` key (the sidebar's two New session
/// buttons, say) is numbered the same in both and compared exactly.
fn is_tool_row(key: &str, role: &str) -> bool {
    let (base, repeated) = base_key(key);
    repeated && role == "button" && TOOL_ROW_BASES.iter().any(|prefix| base.starts_with(prefix))
}

/// The model picker's list rows. `List` virtualizes its rows, so the dump renders only the
/// viewport's prefix of this run while the reference DOM holds all of them.
pub const POPOVER_LIST_PREFIX: &str = "popover/Models/";

/// A model-list row under the `List` virtualizer.
fn is_popover_list_row(key: &str, role: &str) -> bool {
    let (base, _) = base_key(key);
    base.starts_with(POPOVER_LIST_PREFIX) && matches!(role, "option" | "button")
}

/// One thing the order check matches: an exact key, a tool row, or a virtualized popover row.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Exact(String),
    Row(String),
    VirtualRow(String),
}

/// Which virtualization rule a run follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind {
    /// Chat tool rows: the reference window may start anywhere in the dump's full run.
    ToolWindow,
    /// Popover list rows: the dump renders a prefix of the reference's full run.
    PopoverPrefix,
}

/// A key the reference lists in order, or its rows as one unbroken run.
#[derive(Debug)]
enum Segment {
    Exact(String),
    Run(RunKind, Vec<Token>),
}

fn token(key: &str, role: &str) -> Token {
    if is_tool_row(key, role) {
        Token::Row(base_key(key).0.to_owned())
    } else if is_popover_list_row(key, role) {
        Token::VirtualRow(base_key(key).0.to_owned())
    } else {
        Token::Exact(key.to_owned())
    }
}

/// Reference controls missing from the dump, and regions whose shared controls are out of
/// order. Every region but the chat column compares exactly, with the popover list's rendered
/// prefix allowed. The chat column's reference is a window of React's virtualized message
/// list, so its controls (message actions included) must appear as one contiguous window of
/// the dump's chat column, in order.
pub fn control_differences(
    expected: &Value,
    actual: &Value,
    skipped: &HashSet<(String, String)>,
) -> Vec<String> {
    REGIONS
        .iter()
        .filter_map(|&region| {
            let wanted: Vec<Token> = controls(expected, region)
                .iter()
                .filter(|(key, _)| !skipped.contains(&(region.to_owned(), key.clone())))
                .map(|(key, role)| token(key, role))
                .collect();
            let dumped: Vec<Token> = controls(actual, region)
                .iter()
                .map(|(key, role)| token(key, role))
                .collect();
            if region == "chatColumn" {
                chat_column_difference(&wanted, &dumped)
            } else {
                region_order(region, &wanted, &dumped)
            }
        })
        .collect()
}

/// What is wrong with one region, if anything.
fn region_order(region: &str, wanted: &[Token], dumped: &[Token]) -> Option<String> {
    // A virtualized popover list renders a prefix, so unresolved `VirtualRow`s are covered by
    // the run check below; every other token must appear.
    let missing: Vec<&Token> = wanted
        .iter()
        .filter(|want| !matches!(want, Token::VirtualRow(_)) && !dumped.contains(want))
        .collect();
    if !missing.is_empty() {
        return Some(format!("{region}: missing {missing:?}"));
    }
    let shared: Vec<&Token> = dumped
        .iter()
        .filter(|got| matches!(got, Token::Row(_) | Token::VirtualRow(_)) || wanted.contains(got))
        .collect();
    (!in_order(&segments(wanted), dumped))
        .then(|| format!("{region}: order differs\n  expected {wanted:?}\n  got      {shared:?}"))
}

/// Consecutive rows of one virtualization rule become one run.
fn segments(wanted: &[Token]) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    for token in wanted {
        let kind = match token {
            Token::Row(_) => Some(RunKind::ToolWindow),
            Token::VirtualRow(_) => Some(RunKind::PopoverPrefix),
            Token::Exact(key) => {
                segments.push(Segment::Exact(key.clone()));
                None
            }
        };
        if let Some(kind) = kind {
            push_run(&mut segments, kind, token);
        }
    }
    segments
}

fn push_run(segments: &mut Vec<Segment>, kind: RunKind, token: &Token) {
    match segments.last_mut() {
        Some(Segment::Run(existing, run)) if *existing == kind => run.push(token.clone()),
        _ => segments.push(Segment::Run(kind, vec![token.clone()])),
    }
}

/// Whether the segments appear in the dump in order, each run contiguous. Taking the earliest
/// match of each segment leaves the most room for the rest, so a greedy scan is exact.
fn in_order(segments: &[Segment], dumped: &[Token]) -> bool {
    let mut from = 0;
    for segment in segments {
        let found = match segment {
            Segment::Exact(key) => dumped[from..]
                .iter()
                .position(|got| *got == Token::Exact(key.clone()))
                .map(|at| from + at + 1),
            Segment::Run(RunKind::ToolWindow, run) => dumped[from..]
                .windows(run.len())
                .position(|window| window == run.as_slice())
                .map(|at| from + at + run.len()),
            Segment::Run(RunKind::PopoverPrefix, run) => {
                // The dump renders the viewport's prefix of the reference's list.
                let matched = run
                    .iter()
                    .zip(&dumped[from..])
                    .take_while(|(want, got)| want == got)
                    .count();
                (matched > 0).then_some(from + matched)
            }
        };
        match found {
            Some(next) => from = next,
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A region of `(key, role)` controls as a layout would hold it.
    fn layout_of(region: &str, controls: &[(&str, &str)]) -> Value {
        let controls: Vec<Value> = controls
            .iter()
            .map(|(key, role)| serde_json::json!({"key": key, "role": role}))
            .collect();
        serde_json::json!({"regions": {region: {"controls": controls}}})
    }

    fn order_differences(
        region: &str,
        wanted: &[(&str, &str)],
        dumped: &[(&str, &str)],
    ) -> Vec<String> {
        let expected = layout_of(region, wanted);
        let actual = layout_of(region, dumped);
        control_differences(&expected, &actual, &HashSet::new())
    }

    const B: &str = "button";

    #[test]
    fn tool_rows_match_a_window_of_the_dump_in_their_place() {
        let row = |pattern: &str, n: usize| format!("chatColumn/Find Files {pattern}#{n}");
        let (a1, b1, a2, b2, a3) = (
            row("a", 1),
            row("b", 1),
            row("a", 2),
            row("b", 2),
            row("a", 3),
        );
        let dumped = [
            ("chatColumn/generic", "generic"),
            (a1.as_str(), B),
            (b1.as_str(), B),
            (a2.as_str(), B),
            (b2.as_str(), B),
            (a3.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        // React mounted rows 2 to 4 and numbered them from 1.
        let window = [
            ("chatColumn/generic", "generic"),
            (b1.as_str(), B),
            (a1.as_str(), B),
            (b2.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        assert!(order_differences("chatColumn", &window, &dumped).is_empty());

        // The run moved before the scroller: it differs.
        let moved = [
            (b1.as_str(), B),
            (a1.as_str(), B),
            ("chatColumn/generic", "generic"),
            ("chatColumn/Scroll to bottom", B),
        ];
        assert!(!order_differences("chatColumn", &moved, &dumped).is_empty());
        // Rows out of order, or not one run, differ.
        let broken = [(a1.as_str(), B), (a2.as_str(), B)];
        assert!(!order_differences("chatColumn", &broken, &dumped).is_empty());
        // A name holding `#` that is not an ordinal is a single key.
        assert_eq!(base_key("c/Issue #x"), ("c/Issue #x", false));
    }

    #[test]
    fn popover_list_rows_match_the_rendered_prefix() {
        let wanted = [
            ("popover/Search models", "combobox"),
            ("popover/Add new provider", "button"),
            ("popover/Models/Run on several models", "option"),
            ("popover/Models/FAVORITES", "button"),
            ("popover/Models/A", "option"),
            ("popover/Models/A/Favorite", "button"),
            ("popover/Models/B", "option"),
        ];
        // `List` renders the viewport prefix only.
        let dumped = &wanted[..5];
        assert!(order_differences("popover", &wanted, dumped).is_empty());

        // A missing exact control is still a failure.
        let search = wanted[0];
        let add = wanted[1];
        let run_on = wanted[2];
        let favorites = wanted[3];
        let first = wanted[4];
        let without_add = [search, run_on, favorites, first];
        assert!(!order_differences("popover", &wanted, &without_add).is_empty());
        // The virtualized run must start at the same row, not later in the list.
        let skips_run = [search, add, favorites, first];
        assert!(!order_differences("popover", &wanted, &skips_run).is_empty());
    }

    #[test]
    fn other_repeated_keys_compare_exactly_in_order() {
        let reference = [
            ("sidebar/Chats", B),
            ("sidebar/New session#1", B),
            ("sidebar/Search", B),
            ("sidebar/New session#2", B),
        ];
        assert!(order_differences("sidebar", &reference, &reference).is_empty());

        let moved = [
            ("sidebar/Chats", B),
            ("sidebar/New session#1", B),
            ("sidebar/New session#2", B),
            ("sidebar/Search", B),
        ];
        assert!(!order_differences("sidebar", &reference, &moved).is_empty());
    }
}
