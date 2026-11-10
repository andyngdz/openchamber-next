use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::log_file::resolve_data_dir;

const DATA_DIR_ENV: &str = "OPENCHAMBER_DATA_DIR";
const HOME_ENV: &str = "HOME";

/// How to run the CLI: which entry file, from which directory, with which
/// extra env vars, and how long to wait at start and stop.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// The CLI entry file, usually from [`crate::locate_cli`].
    pub cli_path: PathBuf,
    /// Working directory of the CLI process.
    pub working_dir: PathBuf,
    /// Env vars set on top of this process's env, for example an isolated
    /// `HOME`, `OPENCHAMBER_DATA_DIR` or `OPENCODE_CONFIG_CONTENT`. A later
    /// pair wins over an earlier one with the same key.
    pub extra_env: Vec<(OsString, OsString)>,
    /// How long to wait for the port line.
    pub first_line_timeout: Duration,
    /// How long to wait for the CLI to exit after SIGTERM before SIGKILL.
    pub graceful_timeout: Duration,
}

impl SidecarConfig {
    /// Default wait for the port line.
    pub const DEFAULT_FIRST_LINE_TIMEOUT: Duration = Duration::from_secs(30);
    /// Default wait between SIGTERM and SIGKILL.
    pub const DEFAULT_GRACEFUL_TIMEOUT: Duration = Duration::from_secs(10);

    /// A config with the default timeouts.
    ///
    /// # Examples
    ///
    /// ```
    /// use oc_sidecar::SidecarConfig;
    /// let config = SidecarConfig::new("cli.js".into(), "/".into(), Vec::new());
    /// assert_eq!(config.graceful_timeout, SidecarConfig::DEFAULT_GRACEFUL_TIMEOUT);
    /// ```
    pub fn new(
        cli_path: PathBuf,
        working_dir: PathBuf,
        extra_env: Vec<(OsString, OsString)>,
    ) -> Self {
        Self {
            cli_path,
            working_dir,
            extra_env,
            first_line_timeout: Self::DEFAULT_FIRST_LINE_TIMEOUT,
            graceful_timeout: Self::DEFAULT_GRACEFUL_TIMEOUT,
        }
    }

    /// The data dir the CLI will write its logs under, judged from the env the
    /// CLI will see.
    pub(crate) fn data_dir(&self) -> Option<PathBuf> {
        resolve_data_dir(
            self.child_env(DATA_DIR_ENV).as_deref(),
            self.child_env(HOME_ENV).as_deref(),
            &self.working_dir,
        )
    }

    /// The value the CLI will see for `key`: the last extra pair, else this
    /// process's env.
    fn child_env(&self, key: &str) -> Option<OsString> {
        self.extra_env
            .iter()
            .rev()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
            .or_else(|| env::var_os(key))
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;
