use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::Mutex;

use super::collect_tail;
use crate::log_file::TAIL_LINES;

#[test]
fn keeps_only_the_last_stderr_lines() {
    let text: String = (1..=25).map(|index| format!("line {index}\n\n")).collect();
    let lines = Mutex::new(VecDeque::new());
    collect_tail(Cursor::new(text.into_bytes()), &lines);
    let tail: Vec<String> = lines.into_inner().expect("lock").into_iter().collect();
    assert_eq!(tail.len(), TAIL_LINES);
    assert_eq!(tail.first().map(String::as_str), Some("line 6"));
    assert_eq!(tail.last().map(String::as_str), Some("line 25"));
}
