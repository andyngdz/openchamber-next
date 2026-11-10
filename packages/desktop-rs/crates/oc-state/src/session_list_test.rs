//! Ordering, dedupe and reload rules of the list.

use crate::ids::SessionId;
use crate::session_entry::SessionEntry;
use crate::session_list::SessionList;

fn entry(id: &str, activity: u64, created: u64, pinned: bool) -> SessionEntry {
    SessionEntry::new(SessionId::from(id), None, created, activity)
        .with_split_facts(false, pinned, None, false, None)
}

fn ids(list: &SessionList) -> Vec<&str> {
    list.iter().map(|entry| entry.id.as_str()).collect()
}

#[test]
fn a_pinned_row_sorts_before_the_rest() {
    let mut list = SessionList::default();
    list.insert(entry("ses_busy", 99, 1, false));
    list.insert(entry("ses_pinned", 1, 1, true));

    assert_eq!(ids(&list), ["ses_pinned", "ses_busy"]);
}

#[test]
fn rows_sort_by_activity_then_creation_then_id() {
    let mut list = SessionList::default();
    list.insert(entry("ses_old", 10, 1, false));
    list.insert(entry("ses_new", 20, 1, false));
    list.insert(entry("ses_tie_newer", 20, 5, false));
    list.insert(entry("ses_tie_a", 20, 5, false));

    assert_eq!(
        ids(&list),
        ["ses_tie_a", "ses_tie_newer", "ses_new", "ses_old"]
    );
}

#[test]
fn inserting_the_same_session_keeps_one_row() {
    let mut list = SessionList::default();
    list.insert(entry("ses_a", 1, 1, false));
    list.insert(entry("ses_a", 9, 9, false));

    assert_eq!(list.len(), 1);
    assert_eq!(ids(&list), ["ses_a"]);
}

#[test]
fn a_reload_keeps_newer_activity_and_a_newer_status() {
    let mut list = SessionList::default();
    let mut live = entry("ses_a", 5, 1, false);
    live.record_status(crate::status::SessionStatus::Busy, 10);
    list.insert(live);

    let mut fresh = entry("ses_a", 2, 1, false);
    fresh.record_status(crate::status::SessionStatus::Idle, 5);
    list.replace(vec![fresh]);

    let row = list.get(&SessionId::from("ses_a")).expect("row");
    assert_eq!(row.status, crate::status::SessionStatus::Busy);
    assert_eq!(row.activity, 5);
}
