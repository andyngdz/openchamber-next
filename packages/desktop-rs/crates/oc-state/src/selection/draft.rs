//! Choosing the draft's target: the Chat folder or one project (`useDraftTarget.ts:468-497`).
//!
//! Picking a project makes it active, stamps `lastOpenedAt` with the caller's time
//! (`useProjectsStore.ts:844-849`) and re-applies that project's defaults. Picking Chat
//! leaves `activeProjectId` alone.

use super::patches::activate_project;
use super::value::DraftTarget;
use crate::state::State;

impl State {
    pub(super) fn pick_chat(&mut self) {
        self.selection.draft.target = DraftTarget::Chat;
        let directory = self.directory.clone();
        let selection = self.default_selection(&directory);
        self.selection.draft.selection = selection;
    }

    pub(super) fn pick_project(&mut self, id: &str, now: u64) {
        let Some(directory) = self.project_directory(id) else {
            return;
        };
        let patch = self
            .settings_document
            .value
            .as_mut()
            .and_then(|settings| activate_project(settings, id, now));
        if let Some(patch) = patch {
            self.queue_settings_patch(patch);
        }
        self.selection.draft.target = DraftTarget::Project {
            id: id.to_owned(),
            directory: directory.clone(),
        };
        let selection = self.default_selection(&directory);
        self.selection.draft.selection = selection;
    }

    fn project_directory(&self, id: &str) -> Option<String> {
        let settings = self.settings_document.value.as_ref()?;
        let project = settings
            .projects
            .iter()
            .find(|project| project.id() == Some(id))?;
        let directory = project.path()?;
        Some(directory.to_owned())
    }
}

#[cfg(test)]
#[path = "draft_test.rs"]
mod draft_test;
