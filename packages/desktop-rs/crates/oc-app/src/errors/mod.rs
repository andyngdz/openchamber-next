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
    /// The window closed before the layout dump was written.
    #[error("the window closed before the layout was dumped")]
    DumpNotWritten,
}
