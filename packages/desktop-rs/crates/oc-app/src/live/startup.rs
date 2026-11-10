//! Startup and refresh reads: the shared settings, `GET /api/fs/home`, the global session
//! list page by page, the loaded catalogs and the open session's messages.
//!
//! The global walk shows its first page at once and replaces it with the complete list
//! (`packages/ui/src/stores/useGlobalSessionsStore.ts:790-815`). Each page is tried three
//! times, 500 ms apart (`globalSessions.ts:77-88`); a later failure keeps the rows already
//! shown and marks the list incomplete.

use std::time::Duration;

use futures::channel::mpsc::Sender;
use oc_client::session_page::{PageStep, SessionPager};
use oc_client::types::Page;
use oc_client::types::Session;
use oc_client::{Client, Directory};
use oc_state::{Action, SessionId, SidebarAction};

use super::catalog;
use super::protocol::OpenMessages;
use super::protocol::{Closed, Update, apply, sidebar};

/// The global page size React asks for (`useGlobalSessionsStore.ts:77`).
const GLOBAL_PAGE_LIMIT: u32 = 500;
/// Attempts per page, as `retry` is called (`globalSessions.ts:77-88`).
const PAGE_TRIES: u32 = 3;
const PAGE_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Loads everything a fresh window needs, then the draft's catalog.
pub(crate) async fn load_startup(
    client: &Client,
    catalog_directory: &str,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    load_settings(client, updates).await?;
    load_home(client, updates).await?;
    load_global(client, updates).await?;
    catalog::load(client, catalog_directory, updates).await
}

/// Re-reads what a reconnect may have missed.
pub(crate) async fn refetch(
    client: &Client,
    catalogs: Vec<String>,
    open: Option<OpenMessages>,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    load_settings(client, updates).await?;
    load_global(client, updates).await?;
    for directory in catalogs {
        catalog::load(client, &directory, updates).await?;
    }
    if let Some(open) = open {
        load_messages(client, &open.session, &open.directory, updates).await?;
    }
    Ok(())
}

/// `GET /api/config/settings?surface=desktop`.
async fn load_settings(client: &Client, updates: &mut Sender<Update>) -> Result<(), Closed> {
    let action = match client.settings().await {
        Ok(settings) => SidebarAction::SettingsLoaded(Box::new(settings)),
        Err(error) => {
            SidebarAction::SettingsFailed(format!("Could not load the settings: {error}"))
        }
    };
    sidebar(updates, action).await
}

/// `GET /api/fs/home`.
async fn load_home(client: &Client, updates: &mut Sender<Update>) -> Result<(), Closed> {
    let action = match client.home().await {
        Ok(home) => SidebarAction::HomeLoaded(home),
        Err(error) => SidebarAction::HomeFailed(format!("Could not load the home folder: {error}")),
    };
    sidebar(updates, action).await
}

/// Walks the global session list, showing its first page before the rest arrives.
async fn load_global(client: &Client, updates: &mut Sender<Update>) -> Result<(), Closed> {
    let mut pager = SessionPager::new();
    let mut shown = false;
    loop {
        let page = match page_with_retries(client, pager.cursor()).await {
            Ok(page) => page,
            Err(message) => return sidebar(updates, SidebarAction::PageFailed(message)).await,
        };
        let step = pager.accept(page);
        if !shown {
            shown = true;
            let first = pager.sessions().to_vec();
            sidebar(updates, SidebarAction::PageLoaded(first)).await?;
        }
        if step == PageStep::Done {
            let complete = pager.into_sessions();
            return sidebar(updates, SidebarAction::Complete(complete)).await;
        }
    }
}

/// One page, tried three times 500 ms apart; the last error names the failure.
async fn page_with_retries(client: &Client, cursor: Option<&str>) -> Result<Page<Session>, String> {
    let mut message = String::new();
    for attempt in 0..PAGE_TRIES {
        if attempt > 0 {
            tokio::time::sleep(PAGE_RETRY_DELAY).await;
        }
        match client.session_page(GLOBAL_PAGE_LIMIT, cursor).await {
            Ok(page) => return Ok(page),
            Err(error) => message = format!("Could not load the session list: {error}"),
        }
    }
    Err(message)
}

/// `GET /api/session/:id/message` for one session.
pub(crate) async fn load_messages(
    client: &Client,
    session: &SessionId,
    directory: &str,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let directory = Directory::new(directory.to_owned());
    let action = match client.list_messages(&directory, session.as_str()).await {
        Ok(messages) => Action::MessagesLoaded {
            session: session.clone(),
            messages,
        },
        Err(error) => Action::MessagesFailed {
            session: session.clone(),
            message: format!("Could not load messages: {error}"),
        },
    };
    apply(updates, action).await
}

/// `GET /api/session/:id` for the selection; a failure leaves the cascade's choice in place.
pub(crate) async fn load_record(
    client: &Client,
    session: &SessionId,
    directory: &str,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let directory = Directory::new(directory.to_owned());
    match client.session_record(&directory, session.as_str()).await {
        Ok(record) => {
            let action = oc_state::SelectionAction::SessionRecordLoaded {
                session: session.clone(),
                record,
            };
            super::protocol::selection(updates, action).await
        }
        Err(error) => {
            tracing::warn!(
                target: crate::logging::SERVER,
                session = %session,
                %error,
                "[SERVER] Could not load the session record"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "startup_test.rs"]
mod startup_test;
