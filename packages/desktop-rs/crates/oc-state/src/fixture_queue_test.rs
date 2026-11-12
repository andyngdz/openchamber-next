//! Queue fixtures, read by the queue and send tests.
//!
//! Kept apart from `fixture_test.rs` so neither file passes the test-file size limit.

use oc_client::queue::{MessageQueue, QueueUpdate, TakenAll};
use serde::de::DeserializeOwned;

const FIXTURES: &str = env!("OC_DESKTOP_FIXTURES");

fn read_json<T: DeserializeOwned>(relative: &str) -> T {
    let path = format!("{FIXTURES}/{relative}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// A captured message-queue snapshot (`api/message-queue-<name>.json`).
pub(crate) fn api_queue(name: &str) -> MessageQueue {
    read_json(&format!("api/message-queue-{name}.json"))
}

/// A captured one-session queue update (`api/message-queue-<name>.json`).
pub(crate) fn api_queue_update(name: &str) -> QueueUpdate {
    read_json(&format!("api/message-queue-{name}.json"))
}

/// A captured take-all answer (`api/message-queue-<name>.json`).
pub(crate) fn api_taken_all(name: &str) -> TakenAll {
    read_json(&format!("api/message-queue-{name}.json"))
}
