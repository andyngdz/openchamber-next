//! The routes the window calls: the global list, scoped session calls, the catalog and the
//! shared settings.

use serde_json::Value;

use crate::catalog::{Agent, ConfigSource, Model, OpenCodeConfig, Provider};
use crate::client::Client;
use crate::directory::Directory;
use crate::error::ClientError;
use crate::filesystem::{Home, MkdirRequest, MkdirResult};
use crate::prompt::{PromptAccepted, PromptRequest};
use crate::requests::{
    CreateSessionRequest, SessionRecord, SwitchAgentRequest, SwitchModelRequest,
};
use crate::session_page::{PageStep, SessionPager};
use crate::settings::Settings;
use crate::settings_patch::SettingsPatch;
use crate::types::{Data, Located, Location, Message, ModelRef, Page, Session};
use crate::uri_component::encode_uri_component;

/// Same page size the React UI asks for a scoped list.
const SESSION_LIMIT: u32 = 50;
/// Same global page size the React UI asks for (`globalSessions.ts` callers use 500).
const GLOBAL_SESSION_PAGE_LIMIT: u32 = 500;
/// The shared settings document of the desktop surface.
const SETTINGS: &str = "/api/config/settings?surface=desktop";
const SESSIONS: &str = "/api/session";
const HOME: &str = "/api/fs/home";
const MKDIR: &str = "/api/fs/mkdir";
const CONFIG: &str = "/api/config";
const PROVIDERS: &str = "/api/provider";
const MODELS: &str = "/api/model?types=chat&types=image";
const AGENTS: &str = "/api/agent";
/// The prompt route's suffix under `/api/session/:id`; the steer call sends the same one.
pub(crate) const PROMPT: &str = "/prompt";

impl Client {
    /// `GET /api/fs/home`: where the user's home and Chat folders live.
    pub async fn home(&self) -> Result<Home, ClientError> {
        self.get_unscoped(HOME).await
    }

    /// `GET /api/config/settings?surface=desktop`: the shared settings document.
    pub async fn settings(&self) -> Result<Settings, ClientError> {
        self.get_unscoped(SETTINGS).await
    }

    /// `PUT /api/config/settings?surface=desktop` with a partial document; the server
    /// answers with the whole document.
    pub async fn update_settings(&self, patch: &SettingsPatch) -> Result<Settings, ClientError> {
        let request = self.put_unscoped(SETTINGS).json(patch);
        self.send(request).await
    }

    /// `POST /api/fs/mkdir`: creates the folder of a new Chat session.
    pub async fn mkdir(&self, path: &Directory) -> Result<MkdirResult, ClientError> {
        let body = MkdirRequest {
            path: path.as_str().to_owned(),
        };
        self.send(self.post_unscoped(MKDIR).json(&body)).await
    }

    /// `GET /api/config` for a directory, folded from its sources in order.
    pub async fn config(&self, directory: &Directory) -> Result<OpenCodeConfig, ClientError> {
        let sources: Vec<ConfigSource> = self.get_scoped(CONFIG, directory).await?;
        let mut folded = serde_json::Map::new();
        for source in sources {
            if let Some(Value::Object(info)) = source.info {
                folded.extend(info);
            }
        }
        serde_json::from_value(Value::Object(folded)).map_err(ClientError::Decode)
    }

    /// `GET /api/provider` for a directory.
    pub async fn providers(&self, directory: &Directory) -> Result<Vec<Provider>, ClientError> {
        let located: Located<Vec<Provider>> = self.get_scoped(PROVIDERS, directory).await?;
        Ok(located.data)
    }

    /// `GET /api/model?types=chat&types=image` for a directory.
    pub async fn models(&self, directory: &Directory) -> Result<Vec<Model>, ClientError> {
        let located: Located<Vec<Model>> = self.get_scoped(MODELS, directory).await?;
        Ok(located.data)
    }

    /// `GET /api/agent` for a directory.
    pub async fn agents(&self, directory: &Directory) -> Result<Vec<Agent>, ClientError> {
        let located: Located<Vec<Agent>> = self.get_scoped(AGENTS, directory).await?;
        Ok(located.data)
    }

    /// `GET /api/session/:id`: the session's agent and model, which the list does not carry.
    pub async fn session_record(
        &self,
        directory: &Directory,
        session_id: &str,
    ) -> Result<SessionRecord, ClientError> {
        let record: Data<SessionRecord> = self
            .get_scoped(&session_path(session_id, ""), directory)
            .await?;
        Ok(record.data)
    }

    /// `POST /api/session/:id/model`; the server answers 204.
    pub async fn switch_model(
        &self,
        directory: &Directory,
        session_id: &str,
        model: &ModelRef,
    ) -> Result<(), ClientError> {
        let body = SwitchModelRequest {
            model: model.clone(),
        };
        let path = session_path(session_id, "/model");
        self.send_empty(self.post_scoped(&path, directory).json(&body))
            .await
    }

    /// `POST /api/session/:id/agent`; the server answers 204.
    pub async fn switch_agent(
        &self,
        directory: &Directory,
        session_id: &str,
        agent: &str,
    ) -> Result<(), ClientError> {
        let body = SwitchAgentRequest {
            agent: agent.to_owned(),
        };
        let path = session_path(session_id, "/agent");
        self.send_empty(self.post_scoped(&path, directory).json(&body))
            .await
    }

    /// One page of the global session list: `GET /api/session?limit=<n>[&cursor=<next>]`
    /// with no directory header and no directory query.
    pub async fn session_page(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Page<Session>, ClientError> {
        let mut path = format!("{SESSIONS}?limit={limit}");
        if let Some(cursor) = cursor {
            path.push_str("&cursor=");
            path.push_str(&encode_uri_component(cursor));
        }
        self.get_unscoped(&path).await
    }

    /// Every page of the global session list, deduped and stopped by [`SessionPager`]'s
    /// three rules. A failed page fails the whole walk; a caller that keeps the pages
    /// already shown retries [`Client::session_page`] itself.
    pub async fn list_global_sessions(&self) -> Result<Vec<Session>, ClientError> {
        let mut pager = SessionPager::new();
        loop {
            let page = self
                .session_page(GLOBAL_SESSION_PAGE_LIMIT, pager.cursor())
                .await?;
            if pager.accept(page) == PageStep::Done {
                return Ok(pager.into_sessions());
            }
        }
    }

    /// `GET /api/session` for one directory, one page.
    pub async fn list_sessions(&self, directory: &Directory) -> Result<Vec<Session>, ClientError> {
        let path = format!(
            "{SESSIONS}?limit={SESSION_LIMIT}&directory={}",
            encode_uri_component(directory.as_str())
        );
        let page: Page<Session> = self.get_scoped(&path, directory).await?;
        Ok(page.data)
    }

    /// `POST /api/session` in a directory, carrying the picked agent and model; `variant`
    /// is left out of the model when none is picked.
    pub async fn create_session(
        &self,
        directory: &Directory,
        agent: Option<&str>,
        model: Option<&ModelRef>,
    ) -> Result<Session, ClientError> {
        let body = CreateSessionRequest {
            location: Location {
                directory: directory.as_str().to_owned(),
            },
            agent: agent.map(str::to_owned),
            model: model.cloned(),
        };
        let created: Data<Session> = self
            .send(self.post_scoped(SESSIONS, directory).json(&body))
            .await?;
        Ok(created.data)
    }

    /// `GET /api/session/:id/message`, newest first.
    pub async fn list_messages(
        &self,
        directory: &Directory,
        session_id: &str,
    ) -> Result<Vec<Message>, ClientError> {
        let page: Page<Message> = self
            .get_scoped(&session_path(session_id, "/message"), directory)
            .await?;
        Ok(page.data)
    }

    /// `POST /api/session/:id/prompt` with a freshly minted message id.
    pub async fn send_prompt(
        &self,
        directory: &Directory,
        session_id: &str,
        text: &str,
    ) -> Result<PromptAccepted, ClientError> {
        let body = PromptRequest {
            id: crate::new_message_id(),
            text: text.to_owned(),
            delivery: None,
        };
        let path = session_path(session_id, PROMPT);
        let accepted: Data<PromptAccepted> = self
            .send(self.post_scoped(&path, directory).json(&body))
            .await?;
        Ok(accepted.data)
    }
}

/// `/api/session/:id` with a suffix such as `/prompt`, both segments percent-encoded.
pub(crate) fn session_path(session_id: &str, suffix: &str) -> String {
    format!("/api/session/{}{suffix}", encode_uri_component(session_id))
}

#[cfg(test)]
#[path = "calls_test.rs"]
mod calls_test;
