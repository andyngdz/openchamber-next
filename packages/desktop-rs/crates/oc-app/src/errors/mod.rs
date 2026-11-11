//! Failures of the app's own boundaries: reading a state fixture, writing a layout dump.

use std::path::PathBuf;

/// Why the app could not start or could not write what it was asked for.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The state fixture file could not be read.
    #[error("cannot read the state fixture {path}: {source}")]
    ReadFixture {
        /// File that was read.
        path: PathBuf,
        /// What the file system said.
        source: std::io::Error,
    },
    /// The state fixture file is not a valid step list.
    #[error("the state fixture {path} does not decode: {source}")]
    DecodeFixture {
        /// File that was read.
        path: PathBuf,
        /// Where decoding stopped.
        source: serde_json::Error,
    },
    /// The window never laid out a region, so the dump has no bounds for it.
    #[error("the {0} region was not laid out")]
    RegionMissing(&'static str),
    /// The layout dump could not be encoded.
    #[error("cannot encode the layout dump: {0}")]
    EncodeDump(serde_json::Error),
    /// The layout dump could not be written.
    #[error("cannot write the layout dump to {path}: {source}")]
    WriteDump {
        /// File that was written.
        path: PathBuf,
        /// What the file system said.
        source: std::io::Error,
    },
    /// The platform refused to open the window.
    #[error("cannot open the window: {0}")]
    OpenWindow(String),
    /// The headless window could not be opened, drawn or captured.
    #[error("cannot render the window headless: {0}")]
    RenderScreenshot(String),
    /// The screenshot could not be written.
    #[error("cannot write the screenshot to {path}: {source}")]
    WriteScreenshot {
        /// File that was written.
        path: PathBuf,
        /// What the image encoder said.
        source: image::ImageError,
    },
    /// The I/O runtime or its thread could not start.
    #[error("cannot start the I/O runtime: {0}")]
    StartRuntime(std::io::Error),
    /// The thread that owns the sidecar could not start.
    #[error("cannot start the server thread: {0}")]
    StartServerThread(std::io::Error),
    /// The I/O thread ended before it built its runtime.
    #[error("the I/O thread exited before its runtime started")]
    RuntimeThreadExited,
    /// The project directory could not be resolved to its real path.
    #[error("cannot resolve the directory {path}: {source}")]
    ResolveDirectory {
        /// The directory as given.
        path: PathBuf,
        /// What the file system said.
        source: std::io::Error,
    },
    /// The window closed before the layout dump was written.
    #[error("the window closed before the layout was dumped")]
    DumpNotWritten,
}
