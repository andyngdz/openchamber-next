//! A value that loads on its own, like the settings document or the home info.

use crate::status::LoadStatus;

/// The last answer and how the request that fetched it ended.
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
