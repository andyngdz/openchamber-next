//! Reading the selection state and the inputs that change it without a pick.

use oc_client::SettingsPatch;
use oc_client::settings::Settings;
use oc_client::types::{ModelRef, SessionRecord};

use super::cascade::{ProjectDefaults, resolve_default_selection};
use super::catalog::Catalog;
use super::send::{SendSteps, resolve_send_selection, wire_model};
use super::state::DraftState;
use super::value::{DraftTarget, Selection, SelectionSource};
use crate::ids::SessionId;
use crate::state::State;
use crate::status::LoadStatus;

impl State {
    /// The draft's target and selection.
    pub fn draft(&self) -> &DraftState {
        &self.selection.draft
    }

    /// The selection the composer edits: the open session's, else the draft's.
    pub fn current_selection(&self) -> Option<&Selection> {
        match self.open.as_ref() {
            Some(open) => self.selection.selections.get(&open.id),
            None => Some(&self.selection.draft.selection),
        }
    }

    /// One session's selection, once its record arrived.
    pub fn session_selection(&self, session: &SessionId) -> Option<&Selection> {
        self.selection.selections.get(session)
    }

    /// A directory's catalog, once loaded.
    pub fn catalog(&self, directory: &str) -> Option<&Catalog> {
        self.selection
            .catalogs
            .get(directory)
            .and_then(|slot| slot.value.as_ref())
    }

    /// A directory's catalog load state.
    pub fn catalog_load(&self, directory: &str) -> Option<&LoadStatus> {
        self.selection
            .catalogs
            .get(directory)
            .map(|slot| &slot.load)
    }

    /// A session's last fetched record.
    pub fn session_record(&self, session: &SessionId) -> Option<&SessionRecord> {
        self.selection.records.get(session)
    }

    /// The switch steps a send in `session` must run before its prompt.
    pub fn resolve_send_steps(
        &self,
        session: &SessionId,
        desired: &Selection,
        fresh: &SessionRecord,
    ) -> SendSteps {
        let record_model = self
            .selection
            .records
            .get(session)
            .and_then(|record| record.model.as_ref());
        resolve_send_selection(desired, fresh, record_model)
    }

    pub(super) fn catalog_loaded(&mut self, directory: &str, catalog: Catalog) {
        let slot = self
            .selection
            .catalogs
            .entry(directory.to_owned())
            .or_default();
        slot.value = Some(catalog);
        slot.load = LoadStatus::Ready;
        if self.selection_directory().as_deref() == Some(directory) {
            self.fill_auto_selection();
        }
    }

    pub(super) fn catalog_failed(&mut self, directory: &str, message: String) {
        let slot = self
            .selection
            .catalogs
            .entry(directory.to_owned())
            .or_default();
        slot.load = LoadStatus::Failed { message };
    }

    pub(super) fn session_record_loaded(&mut self, session: &SessionId, record: SessionRecord) {
        self.selection
            .records
            .insert(session.clone(), record.clone());
        let selection = Selection::from_record(&record);
        self.selection.selections.insert(session.clone(), selection);
        if self.is_open(session.as_str()) {
            self.fill_auto_selection();
        }
    }

    /// The server accepted `/model`; its record now holds the wire form sent, so the next
    /// send compares against what the session is really on.
    pub(super) fn session_model_switched(&mut self, session: &SessionId, model: &ModelRef) {
        if let Some(record) = self.selection.records.get_mut(session) {
            record.model = Some(wire_model(model));
        }
    }

    /// The server accepted `/agent`; its record now holds the agent sent.
    pub(super) fn session_agent_switched(&mut self, session: &SessionId, agent: &str) {
        if let Some(record) = self.selection.records.get_mut(session) {
            record.agent = Some(agent.to_owned());
        }
    }

    /// Fills the fields the cascade still owns; a user or record choice stays.
    pub(super) fn fill_auto_selection(&mut self) {
        let Some(directory) = self.selection_directory() else {
            return;
        };
        let fallback = self.default_selection(&directory);
        let slot = self.selection_slot_mut();
        if slot.agent_source == SelectionSource::Auto {
            slot.agent = fallback.agent().map(str::to_owned);
        }
        if slot.model_source == SelectionSource::Auto {
            slot.model = fallback.model().cloned();
            slot.variant = fallback.variant().cloned();
            slot.variant_override = None;
        }
    }

    pub(super) fn default_selection(&self, directory: &str) -> Selection {
        let Some(settings) = self.settings_document.value.as_ref() else {
            return Selection::default();
        };
        resolve_default_selection(
            settings,
            project_defaults(settings, directory),
            self.catalog(directory),
        )
    }

    /// The directory the active selection reads its catalog from.
    pub(super) fn selection_directory(&self) -> Option<String> {
        if self.open.is_some() {
            return Some(
                self.open_entry()
                    .map(|entry| entry.directory.clone())
                    .unwrap_or_else(|| self.directory.clone()),
            );
        }
        match &self.selection.draft.target {
            DraftTarget::Project { directory, .. } => Some(directory.clone()),
            DraftTarget::Chat => Some(self.directory.clone()),
        }
    }

    pub(super) fn selection_slot_mut(&mut self) -> &mut Selection {
        match self.open.as_ref().map(|open| open.id.clone()) {
            Some(id) => self.selection.selections.entry(id).or_default(),
            None => &mut self.selection.draft.selection,
        }
    }

    pub(super) fn open_session_id(&self) -> Option<SessionId> {
        self.open.as_ref().map(|open| open.id.clone())
    }

    pub(crate) fn queue_settings_patch(&mut self, patch: SettingsPatch) {
        if patch.is_empty() {
            return;
        }
        super::patches::merge_patch(&mut self.settings_overlay, patch.clone());
        match &mut self.settings_patch {
            Some(existing) => super::patches::merge_patch(existing, patch),
            None => self.settings_patch = Some(patch),
        }
    }
}

/// The configured default agent, model and variant of the project at `directory`.
pub(super) fn project_defaults<'a>(settings: &'a Settings, directory: &str) -> ProjectDefaults<'a> {
    settings
        .projects
        .iter()
        .find(|project| project.path() == Some(directory))
        .map(|project| ProjectDefaults {
            agent: project.default_agent(),
            model: project.default_model(),
            variant: project.default_variant(),
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "access_test.rs"]
mod access_test;
