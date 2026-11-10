//! Decoding and building the version 2 state blocks.

use std::collections::BTreeMap;

use oc_client::Settings;
use oc_client::catalog::{Agent, ConfigSource, Model, OpenCodeConfig, Provider};
use oc_client::filesystem::Home;
use oc_client::types::{ModelRef, Session, SessionRecord};
use oc_state::{Catalog, SelectionAction, SessionId, SidebarAction, State};
use serde::Deserialize;
use serde::de::Error as _;
use serde_json::Value;

use super::steps::{self, Step};
use crate::views::PopoverKind;

/// The only state format this loader reads.
const VERSION: u32 = 2;

/// Parses one fixture file into the state the window shows and the popup to open.
pub(super) fn parse(text: &str) -> Result<(State, Option<PopoverKind>), serde_json::Error> {
    let file: StateFile = serde_json::from_str(text)?;
    if file.version != VERSION {
        return Err(serde_json::Error::custom(format!(
            "state fixture version {}, expected {VERSION}",
            file.version
        )));
    }
    let open = file.open.as_ref().map(Into::into);
    Ok((file.build()?, open))
}

#[derive(Deserialize)]
struct StateFile {
    version: u32,
    home: Home,
    settings: Settings,
    #[serde(default)]
    directories: BTreeMap<String, DirectoryData>,
    #[serde(default)]
    draft: DraftFile,
    #[serde(default)]
    steps: Vec<Step>,
    #[serde(default)]
    open: Option<OpenPopup>,
}

#[derive(Deserialize)]
struct DirectoryData {
    #[serde(default)]
    config: Vec<ConfigSource>,
    #[serde(default)]
    providers: Vec<Provider>,
    #[serde(default)]
    models: Vec<Model>,
    #[serde(default)]
    agents: Vec<Agent>,
    #[serde(default)]
    sessions: Vec<SessionEntry>,
}

/// A session list entry plus the record fields the scoped session answer carries.
#[derive(Deserialize)]
struct SessionEntry {
    #[serde(flatten)]
    session: Session,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    model: Option<ModelRef>,
}

#[derive(Default, Deserialize)]
struct DraftFile {
    #[serde(default)]
    target: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum OpenPopup {
    ProjectPopover,
    ModelPopover,
    AgentMenu,
    ThinkingMenu,
}

impl From<&OpenPopup> for PopoverKind {
    fn from(popup: &OpenPopup) -> Self {
        match popup {
            OpenPopup::ProjectPopover => Self::Project,
            OpenPopup::ModelPopover => Self::Model,
            OpenPopup::AgentMenu => Self::Agent,
            OpenPopup::ThinkingMenu => Self::Thinking,
        }
    }
}

impl StateFile {
    /// Feeds the fixture through the live reducer path. A failed connection keeps the failure
    /// screen: its settings and session blocks are not drawn over the error.
    fn build(self) -> Result<State, serde_json::Error> {
        let mut state = State::new(self.directory());
        if !self.connection_failed() {
            self.load_sidebar(&mut state);
            self.load_catalogs(&mut state)?;
            if self.draft.target != "project" {
                state.apply_selection(SelectionAction::PickChat);
            }
        }
        for action in self.steps.into_iter().flat_map(steps::actions) {
            state.apply(action);
        }
        Ok(state)
    }

    fn load_sidebar(&self, state: &mut State) {
        state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(
            self.settings.clone(),
        )));
        state.apply_sidebar(SidebarAction::HomeLoaded(self.home.clone()));
        state.apply_sidebar(SidebarAction::Complete(self.sessions()));
    }

    fn load_catalogs(&self, state: &mut State) -> Result<(), serde_json::Error> {
        for (directory, data) in &self.directories {
            let config = fold_config(&data.config)?;
            let catalog = Catalog::from_parts(
                data.providers.clone(),
                data.models.clone(),
                data.agents.clone(),
                config,
            );
            state.apply_selection(SelectionAction::CatalogLoaded {
                directory: directory.clone(),
                catalog,
            });
            for entry in &data.sessions {
                let record = SessionRecord {
                    id: entry.session.id.clone(),
                    agent: entry.agent.clone(),
                    model: entry.model.clone(),
                };
                if record.agent.is_some() || record.model.is_some() {
                    state.apply_selection(SelectionAction::SessionRecordLoaded {
                        session: SessionId::from(entry.session.id.as_str()),
                        record,
                    });
                }
            }
        }
        Ok(())
    }

    /// The active project's path, else the first project, else the home directory.
    fn directory(&self) -> String {
        let project = self
            .settings
            .active_project_id
            .as_deref()
            .and_then(|id| self.settings.projects.iter().find(|p| p.id() == Some(id)))
            .or_else(|| self.settings.projects.first());
        project
            .and_then(|project| project.path())
            .unwrap_or(&self.home.home)
            .to_owned()
    }

    /// Whether a step reports a server that never came up or exited.
    fn connection_failed(&self) -> bool {
        self.steps.iter().any(Step::failed)
    }

    /// Every directory's sessions, deduped by id; the sidebar re-sorts them.
    fn sessions(&self) -> Vec<Session> {
        let mut sessions: BTreeMap<String, Session> = BTreeMap::new();
        for data in self.directories.values() {
            for entry in &data.sessions {
                sessions
                    .entry(entry.session.id.clone())
                    .or_insert_with(|| entry.session.clone());
            }
        }
        sessions.into_values().collect()
    }
}

/// The OpenCode config fields folded from the sources in order (`Client::config`).
fn fold_config(sources: &[ConfigSource]) -> Result<OpenCodeConfig, serde_json::Error> {
    let mut folded = serde_json::Map::new();
    for source in sources {
        if let Some(Value::Object(info)) = &source.info {
            folded.extend(info.clone());
        }
    }
    serde_json::from_value(Value::Object(folded))
}

#[cfg(test)]
#[path = "state_file_test.rs"]
mod state_file_test;
