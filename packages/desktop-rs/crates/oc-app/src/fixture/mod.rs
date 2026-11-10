//! `fixtures/states/<name>.json` (version 2): the window's inputs and reducer steps.
//!
//! A version 2 file holds the home info, the shared settings, one directory block per
//! directory the capture read (config sources, providers, models, agents and sessions), the
//! draft target, the reducer steps and the popup the state shows. [`StateScript::load`] feeds
//! every block through the same `State` methods the live app uses, so a dump draws exactly
//! what the live app would draw.

use std::path::Path;

use oc_state::State;

use crate::errors::AppError;
use crate::views::PopoverKind;

mod state_file;
mod steps;

/// A decoded state fixture and the popup its window shows.
#[derive(Debug)]
pub(crate) struct StateScript {
    state: State,
    open: Option<PopoverKind>,
}

impl StateScript {
    /// Reads and decodes the fixture file at `path`, as given on the command line.
    pub(crate) fn load(path: &Path) -> Result<Self, AppError> {
        let text = std::fs::read_to_string(path).map_err(|source| AppError::ReadFixture {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text).map_err(|source| AppError::DecodeFixture {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The state the fixture builds.
    #[cfg(test)]
    pub(crate) fn state(&self) -> State {
        self.state.clone()
    }

    /// The state and the popup to show, for the window.
    pub(crate) fn into_parts(self) -> (State, Option<PopoverKind>) {
        (self.state, self.open)
    }

    fn parse(text: &str) -> Result<Self, serde_json::Error> {
        let (state, open) = state_file::parse(text)?;
        Ok(Self { state, open })
    }
}

#[cfg(test)]
#[path = "fixture_test.rs"]
pub(crate) mod fixture_test;
