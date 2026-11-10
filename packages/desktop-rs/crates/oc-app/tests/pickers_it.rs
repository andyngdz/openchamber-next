//! The pickers end to end against a real server: a session starts on the picked agent and
//! model, every reply comes from the model the session is on, the model and variant switches
//! reach the session record, and a Chat session gets its folder under the chats root. The
//! settings write of the sidebar is `settings_it`.
//!
//! Runs only with `OC_DESKTOP_IT=1`, like `sidecar_it`:
//! `OC_DESKTOP_IT=1 cargo test -p oc-app --test pickers_it -- --nocapture`.
//! Each case starts its own isolated sidecar from [`common::Env`] and runs one at a time.

use std::path::Path;
use std::time::{Duration, Instant};

use futures::StreamExt;
use oc_client::types::{Message, ModelRef};
use oc_client::{Client, Directory, EventData, EventStream};

mod common;
mod support;

use common::{Env, POLL, enabled, fail};
use support::{runtime, start, stop};

/// The two provider ids `common::Env` registers the fixture under, and the models with the
/// variants it gives them.
const PROVIDER: &str = "perf";
const SECOND_PROVIDER: &str = "perf-alt";
const VARIANT_MODEL: &str = "stream-600cps";
const SECOND_MODEL: &str = "stream-1200cps";
const AGENT: &str = "it-primary";
const VARIANT: &str = "high";
const PROMPT: &str = "Reply with the fixture text.";

const TURN_LIMIT: Duration = Duration::from_secs(120);
const CATALOG_LIMIT: Duration = Duration::from_secs(60);

fn model_of(provider: &str, id: &str, variant: Option<&str>) -> ModelRef {
    ModelRef {
        id: id.to_owned(),
        provider_id: provider.to_owned(),
        variant: variant.map(str::to_owned),
    }
}

/// `GET /api/agent` answers 503 until the location's plugins are ready; the picked agent
/// must be in the catalog once it answers.
async fn wait_for_catalog(client: &Client, directory: &Directory) {
    let deadline = Instant::now() + CATALOG_LIMIT;
    let mut last: String;
    loop {
        match client.agents(directory).await {
            Ok(agents) => {
                assert!(
                    agents.iter().any(|agent| agent.id == AGENT),
                    "the catalog has no {AGENT}: {agents:?}"
                );
                return;
            }
            Err(error) => last = error.to_string(),
        }
        assert!(
            Instant::now() < deadline,
            "the agent catalog did not answer within {CATALOG_LIMIT:?}: {last}"
        );
        tokio::time::sleep(POLL).await;
    }
}

/// The model the session record is on.
async fn assert_record_model(
    client: &Client,
    directory: &Directory,
    session: &str,
    expected: &ModelRef,
) {
    let record = client
        .session_record(directory, session)
        .await
        .unwrap_or_else(|error| fail("session record", error));
    assert_eq!(
        record.model.as_ref(),
        Some(expected),
        "the session record is not on {expected:?}"
    );
}

/// Creates a session on the fixture agent and `model`, and checks what the record stored.
async fn create_session(client: &Client, directory: &Directory, model: &ModelRef) -> String {
    let session = client
        .create_session(directory, Some(AGENT), Some(model))
        .await
        .unwrap_or_else(|error| fail("create session", error));
    let record = client
        .session_record(directory, &session.id)
        .await
        .unwrap_or_else(|error| fail("session record", error));
    assert_eq!(record.agent.as_deref(), Some(AGENT));
    assert_eq!(
        record.model.as_ref(),
        Some(model),
        "the created record lost the model"
    );
    session.id
}

/// Sends one prompt and follows the session's events to the end of its turn. Returns the
/// model the turn's step named, and checks the reply text streamed as deltas.
async fn send_turn(
    events: &mut EventStream,
    client: &Client,
    directory: &Directory,
    session: &str,
) -> ModelRef {
    client
        .send_prompt(directory, session, PROMPT)
        .await
        .unwrap_or_else(|error| fail("send prompt", error));
    let mut step = None;
    let mut deltas = 0usize;
    let mut finished = false;
    let wait = async {
        while let Some(item) = events.next().await {
            let Ok(event) = item else { continue };
            match event.data {
                EventData::StepStarted(started) if started.session_id == session => {
                    step = Some(started.model);
                }
                EventData::TextDelta(delta) if delta.session_id == session => deltas += 1,
                EventData::ExecutionSucceeded(done) if done.session_id == session => {
                    finished = true;
                    break;
                }
                EventData::ExecutionFailed(failed) if failed.session_id == session => {
                    fail("turn", format!("failed: {:?}", failed.error));
                }
                _ => {}
            }
        }
    };
    if tokio::time::timeout(TURN_LIMIT, wait).await.is_err() {
        fail("turn", format!("no end within {TURN_LIMIT:?}"));
    }
    assert!(finished, "the event stream ended before the turn finished");
    assert!(deltas > 0, "the turn streamed no text delta");
    step.unwrap_or_else(|| fail("event stream", format!("ended before a step of {session}")))
}

/// The model of the newest assistant message of the session.
async fn reply_model(client: &Client, directory: &Directory, session: &str) -> ModelRef {
    let messages = client
        .list_messages(directory, session)
        .await
        .unwrap_or_else(|error| fail("list messages", error));
    messages
        .iter()
        .find_map(|message| match message {
            Message::Assistant { model, .. } => Some(model.clone()),
            _ => None,
        })
        .unwrap_or_else(|| fail("message list", "no assistant message"))
}

/// The id and provider of the reply, which is all a message carries of its model; the
/// variant lives on the session record.
fn assert_reply_model(actual: &ModelRef, expected: &ModelRef) {
    assert_eq!(
        (actual.id.as_str(), actual.provider_id.as_str()),
        (expected.id.as_str(), expected.provider_id.as_str()),
        "the reply did not come from {expected:?}"
    );
}

/// Switches the open session, checks the record, prompts, and checks the reply. `stored` is
/// what the record holds afterwards, which differs from the request for Default: the server
/// names an unset variant `default`.
async fn switch_and_reply(
    client: &Client,
    directory: &Directory,
    events: &mut EventStream,
    session: &str,
    request: &ModelRef,
    stored: &ModelRef,
) {
    client
        .switch_model(directory, session, request)
        .await
        .unwrap_or_else(|error| fail("switch model", error));
    assert_record_model(client, directory, session, stored).await;
    let step = send_turn(events, client, directory, session).await;
    assert_reply_model(&step, request);
    assert_reply_model(&reply_model(client, directory, session).await, request);
}

#[test]
fn a_session_sends_from_the_picked_model_and_keeps_every_switch() {
    if !enabled() {
        return;
    }
    let env = Env::new();
    let sidecar = start(&env);
    let client = Client::new(sidecar.port()).unwrap_or_else(|error| fail("client", error));
    let directory = Directory::new(env.project.to_string_lossy().into_owned());
    runtime().block_on(async {
        client
            .check_auth()
            .await
            .unwrap_or_else(|error| fail("auth", error));
        wait_for_catalog(&client, &directory).await;
        let mut events = client
            .events()
            .await
            .unwrap_or_else(|error| fail("open events", error));
        let picked = model_of(PROVIDER, VARIANT_MODEL, Some(VARIANT));
        let session = create_session(&client, &directory, &picked).await;
        let step = send_turn(&mut events, &client, &directory, &session).await;
        assert_reply_model(&step, &picked);
        assert_reply_model(&reply_model(&client, &directory, &session).await, &picked);

        let other = model_of(SECOND_PROVIDER, SECOND_MODEL, Some("medium"));
        switch_and_reply(&client, &directory, &mut events, &session, &other, &other).await;
        let variant = model_of(SECOND_PROVIDER, SECOND_MODEL, Some("low"));
        switch_and_reply(
            &client,
            &directory,
            &mut events,
            &session,
            &variant,
            &variant,
        )
        .await;
        // Default sends no variant at all; the server records its own name for that.
        let default = model_of(PROVIDER, VARIANT_MODEL, None);
        let stored = model_of(PROVIDER, VARIANT_MODEL, Some("default"));
        switch_and_reply(
            &client,
            &directory,
            &mut events,
            &session,
            &default,
            &stored,
        )
        .await;
    });
    stop(sidecar);
}

/// `live/send.rs` is crate-private and needs the window's update channel, so this mirrors
/// its two calls for a Chat draft: `mkdir` for the folder the app would open, then a create
/// inside it.
#[test]
fn a_chat_session_gets_its_folder_under_the_chats_root() {
    if !enabled() {
        return;
    }
    let env = Env::new();
    let sidecar = start(&env);
    let client = Client::new(sidecar.port()).unwrap_or_else(|error| fail("client", error));
    let root = env.root.to_string_lossy().into_owned();
    runtime().block_on(async {
        client
            .check_auth()
            .await
            .unwrap_or_else(|error| fail("auth", error));
        let home = client
            .home()
            .await
            .unwrap_or_else(|error| fail("home", error));
        assert!(
            home.chats_root.starts_with(&root),
            "the chats root is outside the isolated root: {}",
            home.chats_root
        );
        let folder = oc_app::chat_folder_for(&home.chats_root, &uuid::Uuid::new_v4().to_string());
        let created = client
            .mkdir(&Directory::new(folder.clone()))
            .await
            .unwrap_or_else(|error| fail("mkdir", error));
        assert!(created.success, "mkdir did not create {folder}");
        assert!(Path::new(&folder).is_dir(), "no chat folder at {folder}");
        let session = client
            .create_session(&Directory::new(folder.clone()), None, None)
            .await
            .unwrap_or_else(|error| fail("create chat session", error));
        let listed = client
            .list_global_sessions()
            .await
            .unwrap_or_else(|error| fail("global sessions", error));
        let chat = listed
            .iter()
            .find(|entry| entry.id == session.id)
            .unwrap_or_else(|| fail("global sessions", format!("{} is not listed", session.id)));
        assert_eq!(chat.location.directory, folder);
    });
    stop(sidecar);
}
