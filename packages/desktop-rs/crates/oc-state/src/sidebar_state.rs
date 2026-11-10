//! The sidebar's inputs: the shared settings, `GET /api/fs/home`, and the global session list.
//!
//! The global walk shows its first page at once and replaces it with the complete list
//! (`packages/ui/src/stores/globalSessions.ts:51-120`); a later failure keeps the rows already
//! shown and marks the list incomplete (design, Failure behavior).

use oc_client::filesystem::Home;
use oc_client::settings::Settings;
use oc_client::types::Session;
use oc_client::{ProjectPatch, SettingsPatch};

use crate::load::session_entry;
use crate::session_entry::SessionEntry;
use crate::session_list::SessionList;
use crate::sidebar::{ChatRoots, MoreKey, SidebarModel, project_index, sections};
use crate::state::{State, load_error};
use crate::status::LoadStatus;

/// A value that loads on its own, like the settings document or the home info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Loaded<T> {
    /// The last answer; a failed refresh keeps it.
    pub(crate) value: Option<T>,
    /// How the last request ended.
    pub(crate) load: LoadStatus,
}

impl<T> Default for Loaded<T> {
    fn default() -> Self {
        Self {
            value: None,
            load: LoadStatus::Loading,
        }
    }
}

/// Where the global walk stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum GlobalStatus {
    /// No page has answered yet.
    #[default]
    Loading,
    /// The first page is shown while later pages load.
    ShowingFirstPage,
    /// The complete list has replaced the first page.
    Complete,
    /// A later page failed; the rows already shown stay.
    Incomplete,
    /// The first page failed; the sidebar shows this with Retry.
    Failed {
        /// Error text shown in the sidebar.
        message: String,
    },
}

/// The global session list and how far its walk has got.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GlobalSessions {
    /// The sessions accepted so far, in sidebar order.
    pub(crate) list: SessionList,
    /// The walk's state.
    pub(crate) status: GlobalStatus,
}

/// One change to the sidebar's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarAction {
    /// `GET /api/config/settings?surface=desktop` answered.
    SettingsLoaded(Box<Settings>),
    /// Settings failed; the sidebar shows the error with Retry.
    SettingsFailed(String),
    /// `GET /api/fs/home` answered.
    HomeLoaded(Home),
    /// Home info failed; the sidebar shows the error with Retry.
    HomeFailed(String),
    /// One page of the global list arrived; the first page is shown at once.
    PageLoaded(Vec<Session>),
    /// The walk finished; the complete list replaces the pages shown.
    Complete(Vec<Session>),
    /// A page failed; the first page shows an error, a later one marks the list incomplete.
    PageFailed(String),
    /// The user revealed one more page of a row list.
    ShowMore(MoreKey),
    /// The user clicked a project header; queued for the settings writer.
    ToggleProjectCollapsed(String),
}

impl State {
    /// Applies one sidebar change. The input actions are separate from [`super::Action`] so the
    /// spike's single-directory callers keep their exhaustive matches.
    pub fn apply_sidebar(&mut self, action: SidebarAction) {
        match action {
            SidebarAction::SettingsLoaded(settings) => {
                self.settings_document.value = Some(*settings);
                self.settings_document.load = LoadStatus::Ready;
            }
            SidebarAction::SettingsFailed(message) => {
                self.settings_document.load = LoadStatus::Failed { message };
            }
            SidebarAction::HomeLoaded(home) => {
                self.home_document.value = Some(home);
                self.home_document.load = LoadStatus::Ready;
            }
            SidebarAction::HomeFailed(message) => {
                self.home_document.load = LoadStatus::Failed { message };
            }
            SidebarAction::PageLoaded(page) => self.apply_page(page),
            SidebarAction::Complete(list) => self.complete_walk(list),
            SidebarAction::PageFailed(message) => self.fail_page(message),
            SidebarAction::ShowMore(key) => {
                *self.show_more.entry(key).or_default() += 1;
            }
            SidebarAction::ToggleProjectCollapsed(id) => self.toggle_project(&id),
        }
    }

    /// The whole sidebar, derived from the current inputs.
    pub fn sidebar_model(&self) -> SidebarModel {
        let mut model = SidebarModel {
            sections: Vec::new(),
            error: self.sidebar_error().map(str::to_owned),
            loading: self.sidebar_loading(),
            complete: matches!(self.global.status, GlobalStatus::Complete),
            incomplete: matches!(self.global.status, GlobalStatus::Incomplete),
        };
        if model.error.is_none()
            && let (Some(settings), Some(home)) =
                (&self.settings_document.value, &self.home_document.value)
        {
            model.sections = sections(settings, home, &self.global.list, &self.show_more);
        }
        model
    }

    /// The shared settings, once loaded.
    pub fn settings(&self) -> Option<&Settings> {
        self.settings_document.value.as_ref()
    }

    /// The home info, once loaded.
    pub fn home(&self) -> Option<&Home> {
        self.home_document.value.as_ref()
    }

    /// The global session list as accepted so far.
    pub fn global_sessions(&self) -> &SessionList {
        &self.global.list
    }

    /// Takes the settings patch a collapse click queued.
    pub fn take_settings_patch(&mut self) -> Option<SettingsPatch> {
        self.settings_patch.take().filter(|patch| !patch.is_empty())
    }

    /// The sidebar shows this error instead of a project list.
    pub(crate) fn sidebar_error(&self) -> Option<&str> {
        load_error(&self.settings_document.load)
            .or_else(|| load_error(&self.home_document.load))
            .or(match &self.global.status {
                GlobalStatus::Failed { message } => Some(message.as_str()),
                _ => None,
            })
            .or_else(|| load_error(&self.sidebar))
    }

    /// Whether a new session in `directory` belongs to one of the sidebar's groups.
    pub(crate) fn sidebar_admits(&self, directory: &str) -> bool {
        if directory == self.directory {
            return true;
        }
        let (Some(settings), Some(home)) =
            (&self.settings_document.value, &self.home_document.value)
        else {
            return false;
        };
        let Some(roots) = ChatRoots::from_home(home) else {
            return false;
        };
        roots.contains(directory) || project_index(directory, &settings.projects).is_some()
    }

    fn sidebar_loading(&self) -> bool {
        matches!(self.settings_document.load, LoadStatus::Loading)
            || matches!(self.home_document.load, LoadStatus::Loading)
            || matches!(self.global.status, GlobalStatus::Loading)
    }

    fn apply_page(&mut self, page: Vec<Session>) {
        for session in &page {
            self.global.list.insert(session_entry(session));
        }
        if matches!(
            self.global.status,
            GlobalStatus::Loading | GlobalStatus::Failed { .. }
        ) {
            self.global.status = GlobalStatus::ShowingFirstPage;
        }
    }

    fn complete_walk(&mut self, list: Vec<Session>) {
        let entries: Vec<SessionEntry> = list.iter().map(session_entry).collect();
        self.global.list.replace(entries);
        self.global.status = GlobalStatus::Complete;
    }

    fn fail_page(&mut self, message: String) {
        self.global.status = match self.global.status {
            GlobalStatus::ShowingFirstPage => GlobalStatus::Incomplete,
            _ => GlobalStatus::Failed { message },
        };
    }

    /// Flips `sidebarCollapsed` on the raw project and queues the whole array for the writer
    /// (`useSessionProjectViewState.ts:79-103`), keeping every field this app does not change.
    fn toggle_project(&mut self, id: &str) {
        let Some(settings) = self.settings_document.value.as_mut() else {
            return;
        };
        let Some(index) = settings
            .projects
            .iter()
            .position(|project| project.id() == Some(id))
        else {
            return;
        };
        let collapsed = !settings.projects[index]
            .sidebar_collapsed()
            .unwrap_or(false);
        let patch = ProjectPatch {
            sidebar_collapsed: Some(collapsed),
            ..ProjectPatch::default()
        };
        settings.projects[index] = settings.projects[index].with_patch(&patch);
        self.settings_patch = Some(SettingsPatch {
            projects: Some(settings.projects.clone()),
            ..SettingsPatch::default()
        });
    }
}

#[cfg(test)]
#[path = "sidebar_state_test.rs"]
mod sidebar_state_test;
