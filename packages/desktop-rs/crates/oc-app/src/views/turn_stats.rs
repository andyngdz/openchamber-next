//! The Turn stats rows of the Work status card, once a turn has ended.
//!
//! Each row is named by its label; the figure beside it is data, `-` where the state holds
//! none.

use super::turns::Turn;

/// One row: its label, which is also its name, and the figure it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatRow {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

/// Where a row's figure comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Figure {
    /// The turn's duration, from its first step's start to its last step's end.
    Elapsed,
    /// The number of steps in the turn.
    Steps,
    /// A figure `oc-state` does not carry (rates, model and tool time, usage, cost).
    Unread,
}

const ROWS: [(&str, Figure); 10] = [
    ("Response", Figure::Unread),
    ("Whole turn", Figure::Unread),
    ("Elapsed", Figure::Elapsed),
    ("Model time", Figure::Unread),
    ("Tool time", Figure::Unread),
    ("Average TTFT", Figure::Unread),
    ("Steps", Figure::Steps),
    ("Tokens", Figure::Unread),
    ("Cache", Figure::Unread),
    ("Cost", Figure::Unread),
];
const NO_FIGURE: &str = "-";

/// The rows for an ended turn.
pub(crate) fn stat_rows(turn: &Turn<'_>) -> Vec<StatRow> {
    ROWS.iter()
        .map(|&(label, figure)| StatRow {
            label,
            value: match figure {
                Figure::Elapsed => turn
                    .meta()
                    .map_or_else(|| NO_FIGURE.to_owned(), |meta| meta.duration),
                Figure::Steps => turn.steps.len().to_string(),
                Figure::Unread => NO_FIGURE.to_owned(),
            },
        })
        .collect()
}

#[cfg(test)]
#[path = "turn_stats_test.rs"]
mod turn_stats_test;
