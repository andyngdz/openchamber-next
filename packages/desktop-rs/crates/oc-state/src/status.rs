//! The closed states the layout shows: server connection, region loads, composer, session activity.

/// Where the app stands with its server; shown in the header where the context percentage sits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    /// The sidecar is starting or the auth check is running.
    #[default]
    Starting,
    /// Authenticated and the event stream is open.
    Connected,
    /// The event stream dropped; `attempt` counts reconnects since the last success, from 1.
    Reconnecting {
        /// Reconnect attempt, from 1.
        attempt: u32,
    },
    /// The server never came up, or the auth check failed.
    Failed(ServerFailure),
    /// The server process exited while the app was running.
    ServerExited(ServerExit),
}

/// Why the server could not be reached, with the output that explains it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerFailure {
    /// One line naming the failure.
    pub message: String,
    /// Last lines the CLI wrote to stderr.
    pub stderr_tail: Vec<String>,
    /// Last lines of the server log file written since the spawn.
    pub log_tail: Vec<String>,
}

/// How the server process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerExit {
    /// Exited with this code.
    Code(i32),
    /// Killed by this signal number.
    Signal(i32),
    /// The platform reported neither.
    Unknown,
}

/// The parts of the window that load data on their own and show their own error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// The sidebar's session list.
    Sidebar,
    /// The open session's messages in the chat column.
    Messages,
    /// The composer; its error shows above it.
    Composer,
    /// Creating a session; its error shows in the sidebar, and its Retry creates again.
    NewSession,
}

/// Load state of the session list or of the open session's messages.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LoadStatus {
    /// A request is in flight, or none has answered yet.
    #[default]
    Loading,
    /// The last request answered and its data is in the state.
    Ready,
    /// The last request failed; the region shows the message and a retry button.
    Failed {
        /// Error text shown in the region.
        message: String,
    },
}

/// What the composer is doing with the draft.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ComposerStatus {
    /// Nothing in flight.
    #[default]
    Idle,
    /// The prompt request is in flight; the draft stays until the server accepts it.
    Sending,
    /// The prompt request failed; the draft is kept and the error shows above the composer.
    Failed {
        /// Error text shown above the composer.
        message: String,
    },
}

/// Whether a session is running a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionStatus {
    /// No turn is running.
    #[default]
    Idle,
    /// A turn is running.
    Busy,
    /// The last turn ended in an error.
    Error,
}
