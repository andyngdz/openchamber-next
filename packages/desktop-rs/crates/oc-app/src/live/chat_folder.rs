//! The folder of a new Chat session: `<chatsRoot>/YYYY-MM-DD/session-<uuid v4>`
//! (`packages/ui/src/lib/chatDirectories.ts:109-118`). The date is the local day, and the
//! server creates the folder with `POST /api/fs/mkdir` before the session is created in it.

use chrono::{DateTime, Datelike, Local};
use oc_client::filesystem::Home;
use uuid::Uuid;

/// The folder a new Chat session goes in, or `None` while the server has named neither a
/// Chats root nor a home directory.
pub(crate) fn new_chat_folder(home: &Home) -> Option<String> {
    let root = chats_root(home)?;
    Some(chat_folder_for(&root, &Uuid::new_v4().to_string()))
}

/// The folder a Chat session with `uuid` goes in today: `<root>/<local day>/session-<uuid>`
/// (`chatDirectories.ts:109-118`). Exposed so the integration test builds the exact path.
pub fn chat_folder_for(root: &str, uuid: &str) -> String {
    let now = Local::now();
    format!(
        "{}/{}/session-{uuid}",
        root.trim_end_matches('/'),
        date(&now)
    )
}

/// The configured Chats root, else the legacy `<home>/.config/openchamber/chats` that
/// `legacyRootForHome` derives (`chatDirectories.ts:16-19`).
fn chats_root(home: &Home) -> Option<String> {
    let configured = home.chats_root.trim_end_matches('/');
    if !configured.is_empty() {
        return Some(configured.to_owned());
    }
    let base = home.home.trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/.config/openchamber/chats"))
}

fn date(now: &DateTime<Local>) -> String {
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

#[cfg(test)]
#[path = "chat_folder_test.rs"]
mod chat_folder_test;
