use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;

use nix::errno::Errno;
use nix::unistd::Pid;

/// Why the first stdout line of the CLI is not a usable port.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PortLineError {
    /// The line holds nothing but whitespace.
    #[error("the port line is empty")]
    Empty,
    /// The line carries a generated UI password; the spike runs without one.
    /// The password itself is never stored.
    #[error("the server printed a UI password, and the sidecar runs without one")]
    PasswordLine,
    /// The line is not a single port number from 1 to 65535.
    #[error("the port line is not a port number: {line:?}")]
    NotAPort {
        /// The line as printed, without its line break.
        line: String,
    },
}

/// What a CLI that failed to start left behind, for the error screen.
#[derive(Debug, Clone, Default)]
pub struct StartupDiagnostics {
    /// Exit status, when the CLI exited or was stopped after the failure.
    pub exit_status: Option<ExitStatus>,
    /// The last lines the CLI wrote to stderr.
    pub stderr_tail: Vec<String>,
    /// The server log file for the attempted port, when the data dir is known.
    pub log_path: Option<PathBuf>,
    /// The last lines written to the log file since the spawn.
    pub log_tail: Vec<String>,
}

/// Every way starting or stopping the sidecar fails.
#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    /// Neither `OPENCHAMBER_CLI` nor a walk up from the searched directories
    /// found `packages/web/bin/cli.js`.
    #[error("OpenChamber CLI not found; set OPENCHAMBER_CLI (searched up from {searched:?})")]
    CliNotFound {
        /// Directories the walk started from.
        searched: Vec<PathBuf>,
    },
    /// Binding `127.0.0.1:0` to pick a port failed.
    #[error("could not pick a free local port")]
    FreePort(#[source] io::Error),
    /// The OS refused to start the process, for example `node` is missing.
    #[error("could not start {program}")]
    Spawn {
        /// The program that failed to start.
        program: String,
        /// The OS error.
        #[source]
        source: io::Error,
    },
    /// Reading the CLI's stdout failed.
    #[error("could not read the server's stdout")]
    ReadStdout(#[source] io::Error),
    /// No line arrived on stdout in time; the CLI was stopped.
    #[error("the server printed no port line within {waited:?}")]
    Timeout {
        /// The timeout that ran out.
        waited: Duration,
        /// What the CLI left behind.
        diagnostics: StartupDiagnostics,
    },
    /// The CLI exited before it printed a line.
    #[error("the server exited before printing its port")]
    ExitedEarly {
        /// What the CLI left behind.
        diagnostics: StartupDiagnostics,
    },
    /// Every attempt lost its picked port to another process.
    #[error("another process took the picked port on all {attempts} attempts")]
    PortInUse {
        /// Attempts made.
        attempts: u32,
        /// What the last attempt left behind.
        diagnostics: StartupDiagnostics,
    },
    /// The first stdout line is not a port; the CLI was stopped.
    #[error("the server printed an unusable port line")]
    InvalidPortLine {
        /// Why the line was rejected.
        #[source]
        source: PortLineError,
        /// What the CLI left behind.
        diagnostics: StartupDiagnostics,
    },
    /// Sending a signal to a process failed for a reason other than the
    /// process being gone.
    #[error("could not signal process {pid}")]
    Signal {
        /// The target process.
        pid: Pid,
        /// The OS error.
        #[source]
        source: Errno,
    },
    /// Waiting for the CLI process failed.
    #[error("could not wait for the server process")]
    Wait(#[source] io::Error),
}

impl SidecarError {
    /// The stderr and log tails of a failed start, for errors that have them.
    ///
    /// # Examples
    ///
    /// ```
    /// use oc_sidecar::SidecarError;
    /// let error = SidecarError::CliNotFound { searched: Vec::new() };
    /// assert!(error.diagnostics().is_none());
    /// ```
    pub fn diagnostics(&self) -> Option<&StartupDiagnostics> {
        match self {
            Self::Timeout { diagnostics, .. }
            | Self::ExitedEarly { diagnostics }
            | Self::PortInUse { diagnostics, .. }
            | Self::InvalidPortLine { diagnostics, .. } => Some(diagnostics),
            Self::CliNotFound { .. }
            | Self::FreePort(_)
            | Self::Spawn { .. }
            | Self::ReadStdout(_)
            | Self::Signal { .. }
            | Self::Wait(_) => None,
        }
    }
}
