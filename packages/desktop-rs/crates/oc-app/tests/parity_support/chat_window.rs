//! The chat column's window comparison: React virtualizes the message list, so the
//! reference holds only the messages mounted around the viewport while the dump draws the
//! whole transcript.

use super::Token;

/// The chat column's list chrome: the scroller and the scroll-to-bottom button bracket the
/// message window in the reference. They are mounted whatever the virtualizer renders, so
/// they anchor the window instead of being part of it.
const CHAT_SCROLLER: &str = "chatColumn/generic";
const CHAT_SCROLL_TO_BOTTOM: &str = "chatColumn/Scroll to bottom";

/// Names of the user-message action row (`MessageBody.tsx`, `actionsBlock`).
const CHAT_USER_ACTIONS: [&str; 3] = [
    "chatColumn/Revert to this message",
    "chatColumn/Copy message text",
    "chatColumn/More actions",
];

/// The chat column's reference window, matched against the dump.
///
/// The reference lists only the messages React mounted around the viewport, while the dump
/// draws the whole transcript. Its controls must therefore appear as one contiguous window
/// of the dump's, in order, with tool rows compared by base key because virtualization
/// renumbers them. The scroller, the user action row and the scroll-to-bottom button sit
/// outside that window: the scroller heads it, the user actions follow the scroller in a
/// mounted message, and the scroll button closes the list. Earlier and later messages'
/// controls may surround the window; a control missing inside it still fails.
pub(super) fn chat_column_difference(wanted: &[Token], dumped: &[Token]) -> Option<String> {
    let missing: Vec<&Token> = wanted
        .iter()
        .filter(|want| !dumped.contains(want))
        .collect();
    if !missing.is_empty() {
        return Some(format!("chatColumn: missing {missing:?}"));
    }
    if wanted.is_empty() {
        return None;
    }

    let mut start = 0;
    let mut from = 0;
    let scroller = Token::Exact(CHAT_SCROLLER.to_owned());
    if wanted.first() == Some(&scroller) {
        let Some(at) = dumped.iter().position(|got| *got == scroller) else {
            return Some(format!("chatColumn: missing {CHAT_SCROLLER:?}"));
        };
        from = at + 1;
        start = 1;
    }
    while start < wanted.len() {
        let current = &wanted[start];
        let Token::Exact(key) = current else {
            break;
        };
        if !CHAT_USER_ACTIONS.contains(&key.as_str()) {
            break;
        }
        let Some(at) = dumped[from..].iter().position(|got| got == current) else {
            return Some(format!("chatColumn: missing {current:?}"));
        };
        from += at + 1;
        start += 1;
    }

    let scroll_button = Token::Exact(CHAT_SCROLL_TO_BOTTOM.to_owned());
    let mut end = wanted.len();
    let mut to = dumped.len();
    if wanted.last() == Some(&scroll_button) {
        let Some(at) = dumped[from..].iter().rposition(|got| *got == scroll_button) else {
            return Some(format!("chatColumn: missing {CHAT_SCROLL_TO_BOTTOM:?}"));
        };
        to = from + at;
        end = wanted.len() - 1;
    }

    let body = &wanted[start..end.max(start)];
    if body.is_empty() {
        return None;
    }
    if to < from {
        return Some(format!(
            "chatColumn: controls are not one window in the dump\n  expected {body:?}"
        ));
    }
    let between = &dumped[from..to];
    if between.windows(body.len()).any(|window| window == body) {
        None
    } else {
        Some(format!(
            "chatColumn: controls are not one window in the dump\n  expected {body:?}\n  got      {between:?}"
        ))
    }
}

/// React renders the user action row on every mounted user message. The session reference
/// cannot show it (the virtualizer unmounted that message), so the dump proves the rule for
/// the idle transcript too.
#[test]
fn a_finished_session_still_draws_its_user_actions() {
    let layout = crate::dump("session");
    let names: Vec<&str> = layout["regions"]["chatColumn"]["controls"]
        .as_array()
        .expect("chat column has controls")
        .iter()
        .map(|control| control["name"].as_str().unwrap_or_default())
        .collect();
    let user = [
        "Revert to this message",
        "Copy message text",
        "More actions",
    ];
    assert!(
        names.windows(user.len()).any(|window| window == user),
        "the idle session's rendered user message lost its actions: {names:?}"
    );
}
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::Value;

    use super::super::control_differences;

    const B: &str = "button";

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

    #[test]
    fn chat_column_matches_a_window_of_the_mounted_messages() {
        let row = |pattern: &str, n: usize| format!("chatColumn/Find Files {pattern}#{n}");
        let (a1, b1, b2) = (row("a", 1), row("b", 1), row("b", 2));
        let dumped = [
            ("chatColumn/generic", "generic"),
            ("chatColumn/Revert to this message", B),
            ("chatColumn/Copy message text", B),
            ("chatColumn/More actions", B),
            (a1.as_str(), B),
            (b1.as_str(), B),
            (b2.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        // The reference mounted the user actions and the later rows; the row above
        // the viewport and the actions' neighbours stay in the dump only.
        let wanted = [
            ("chatColumn/generic", "generic"),
            ("chatColumn/Revert to this message", B),
            ("chatColumn/Copy message text", B),
            ("chatColumn/More actions", B),
            (b1.as_str(), B),
            (b2.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        assert!(order_differences("chatColumn", &wanted, &dumped).is_empty());

        // An idle transcript's user message scrolled out of the window: the mounted
        // rows and the meta controls still form one window.
        let idle = [
            ("chatColumn/generic", "generic"),
            (b1.as_str(), B),
            (b2.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        assert!(order_differences("chatColumn", &idle, &dumped).is_empty());
    }

    #[test]
    fn a_control_missing_inside_the_chat_window_fails() {
        let row = |pattern: &str, n: usize| format!("chatColumn/Find Files {pattern}#{n}");
        let (a1, b1) = (row("a", 1), row("b", 1));
        let dumped = [
            ("chatColumn/generic", "generic"),
            ("chatColumn/Revert to this message", B),
            ("chatColumn/Copy message text", B),
            ("chatColumn/More actions", B),
            (a1.as_str(), B),
            (b1.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        // A control the dump never drew, in the middle of the window.
        let missing = [
            ("chatColumn/generic", "generic"),
            (a1.as_str(), B),
            ("chatColumn/Missing", B),
            (b1.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        assert!(!order_differences("chatColumn", &missing, &dumped).is_empty());

        // Another control between two window rows breaks the window, even though
        // every reference key exists.
        let extra = [
            ("chatColumn/generic", "generic"),
            (a1.as_str(), B),
            ("chatColumn/Other", B),
            (b1.as_str(), B),
            ("chatColumn/Scroll to bottom", B),
        ];
        let with_extra = [extra[0], extra[1], extra[3], extra[4]];
        assert!(!order_differences("chatColumn", &with_extra, &extra).is_empty());
    }
}
