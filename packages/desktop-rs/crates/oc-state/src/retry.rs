//! Retrying one failed region (`Action::Retry`).

use crate::state::State;
use crate::status::{ComposerStatus, LoadStatus, Region};

impl State {
    pub(crate) fn retry(&mut self, region: Region) {
        match region {
            Region::Sidebar => self.sidebar = LoadStatus::Loading,
            Region::Messages => {
                if let Some(open) = self.open.as_mut() {
                    open.load = LoadStatus::Loading;
                }
            }
            Region::Composer => self.composer.status = ComposerStatus::Idle,
            Region::NewSession => self.create_error = None,
        }
    }
}
