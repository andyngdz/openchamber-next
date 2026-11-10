//! Pinned and In work membership, the exclusions, and what each section claims.

use serde_json::json;

use super::{WorkPlacement, resolve};
use crate::fixture_test::{home, sidebar_from};
use crate::load::session_entry;
use crate::session_list::SessionList;
use crate::sidebar::section::BuiltinSection;
use crate::sidebar::split::{ChatRoots, split};

const CHAT: &str = "/home/u/.config/openchamber/chats/2026-01-02/s";

fn list() -> SessionList {
    let sessions = [
        json!({"id": "ses_work", "location": {"directory": "/p/app"},
            "time": {"created": 1, "updated": 60},
            "metadata": {"openchamber": {"work": {"state": "open"}}}}),
        json!({"id": "ses_pinned", "location": {"directory": "/p/app"},
            "time": {"created": 1, "updated": 50},
            "metadata": {"openchamber": {"pinned": {"at": 1}}}}),
        json!({"id": "ses_both", "location": {"directory": "/p/app"},
            "time": {"created": 1, "updated": 40},
            "metadata": {"openchamber": {"pinned": {"at": 1}, "work": {"state": "open"}}}}),
        json!({"id": "ses_sub", "location": {"directory": "/p/app"}, "parentID": "ses_work",
            "time": {"created": 1, "updated": 30},
            "metadata": {"openchamber": {"work": {"state": "open"}}}}),
        json!({"id": "ses_archived", "location": {"directory": "/p/app"},
            "time": {"created": 1, "updated": 20, "archived": 20},
            "metadata": {"openchamber": {"work": {"state": "open"}}}}),
        json!({"id": "ses_chat", "location": {"directory": CHAT},
            "time": {"created": 1, "updated": 10},
            "metadata": {"openchamber": {"work": {"state": "open"}}}}),
        json!({"id": "ses_list", "location": {"directory": "/p/app"},
            "time": {"created": 1, "updated": 5},
            "metadata": {"openchamber": {"pinned": {"at": 1, "list": "mine"}}}}),
    ];
    let mut list = SessionList::default();
    for session in sessions {
        list.insert(session_entry(&sidebar_from(session)));
    }
    list
}

fn ids(ids: &[crate::ids::SessionId]) -> Vec<&str> {
    ids.iter().map(crate::ids::SessionId::as_str).collect()
}

fn membership(sections: &[BuiltinSection], placement: WorkPlacement) -> super::Membership {
    let projects: Vec<oc_client::Project> =
        serde_json::from_value(json!([{"id": "app", "path": "/p/app"}])).expect("project");
    let roots = ChatRoots::from_home(&home(json!({"home": "/home/u"}))).expect("roots");
    let list = list();
    let split = split(&list, &projects, &roots);
    resolve(&list, &split, sections, placement)
}

#[test]
fn pinned_and_work_take_their_sessions_out_of_the_project() {
    let membership = membership(
        &[
            BuiltinSection::Chats,
            BuiltinSection::Pinned,
            BuiltinSection::Work,
            BuiltinSection::Projects,
        ],
        WorkPlacement::Claims,
    );

    assert_eq!(ids(&membership.pinned), ["ses_pinned", "ses_both"]);
    assert_eq!(ids(&membership.work), ["ses_work"]);
    assert!(membership.claimed.contains(&"ses_pinned".into()));
    assert!(membership.claimed.contains(&"ses_work".into()));
}

#[test]
fn a_pin_in_a_list_and_a_subagent_stay_in_the_project() {
    let membership = membership(
        &[BuiltinSection::Pinned, BuiltinSection::Work],
        WorkPlacement::Claims,
    );

    assert!(!membership.claimed.contains(&"ses_list".into()));
    assert!(!membership.claimed.contains(&"ses_sub".into()));
}

#[test]
fn keep_work_in_place_leaves_the_session_in_its_project() {
    let membership = membership(
        &[BuiltinSection::Pinned, BuiltinSection::Work],
        WorkPlacement::StaysInPlace,
    );

    assert_eq!(ids(&membership.work), ["ses_work"]);
    assert!(!membership.claimed.contains(&"ses_work".into()));
    assert!(membership.claimed.contains(&"ses_pinned".into()));
}

#[test]
fn without_the_pinned_section_a_work_session_is_not_taken() {
    let membership = membership(
        &[BuiltinSection::Chats, BuiltinSection::Work],
        WorkPlacement::Claims,
    );

    assert!(membership.pinned.is_empty());
    assert_eq!(ids(&membership.work), ["ses_both", "ses_work"]);
    assert!(!membership.claimed.contains(&"ses_pinned".into()));
}
