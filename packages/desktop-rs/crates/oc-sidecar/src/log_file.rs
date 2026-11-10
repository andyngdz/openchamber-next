use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// How many lines of stderr and of the log file a startup error keeps.
pub const TAIL_LINES: usize = 20;

/// Resolves the CLI's data dir the way `packages/web/bin/lib/cli-paths.js:9-14`
/// does: a non-blank `OPENCHAMBER_DATA_DIR`, resolved against the CLI's
/// working directory, else `<home>/.config/openchamber`. Without either value
/// the data dir is unknown.
///
/// # Examples
///
/// ```
/// use std::ffi::OsStr;
/// use std::path::{Path, PathBuf};
/// let dir = oc_sidecar::resolve_data_dir(None, Some(OsStr::new("/home/u")), Path::new("/"));
/// assert_eq!(dir, Some(PathBuf::from("/home/u/.config/openchamber")));
/// ```
pub fn resolve_data_dir(
    data_dir: Option<&OsStr>,
    home: Option<&OsStr>,
    working_dir: &Path,
) -> Option<PathBuf> {
    let explicit = data_dir
        .map(OsStr::to_string_lossy)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(value) = explicit {
        return Some(working_dir.join(value));
    }
    home.filter(|value| !value.is_empty())
        .map(|value| Path::new(value).join(".config").join("openchamber"))
}

/// The server log file the CLI writes for an explicit port
/// (`packages/web/bin/lib/cli-paths.js:16-18`, `:53-55`).
///
/// # Examples
///
/// ```
/// use std::path::{Path, PathBuf};
/// let path = oc_sidecar::log_file_path(Path::new("/data"), 4096);
/// assert_eq!(path, PathBuf::from("/data/logs/openchamber-4096.log"));
/// ```
pub fn log_file_path(data_dir: &Path, port: u16) -> PathBuf {
    data_dir
        .join("logs")
        .join(format!("openchamber-{port}.log"))
}

/// Marks the end of a log file before a spawn, so a failure reports only the
/// lines that spawn wrote.
#[derive(Debug, Clone)]
pub(crate) struct LogCursor {
    path: PathBuf,
    start_len: u64,
}

impl LogCursor {
    /// Records the current length of `path`; a missing file counts as empty.
    pub(crate) fn at_end_of(path: PathBuf) -> Self {
        let start_len = fs::metadata(&path).map_or(0, |meta| meta.len());
        Self { path, start_len }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The last lines written since the cursor was taken. A file shorter than
    /// at the start was rotated by the CLI, so all of it is new.
    pub(crate) fn tail(&self) -> Vec<String> {
        let Ok(bytes) = fs::read(&self.path) else {
            return Vec::new();
        };
        let start = usize::try_from(self.start_len)
            .ok()
            .filter(|start| *start <= bytes.len())
            .unwrap_or(0);
        last_lines(&String::from_utf8_lossy(&bytes[start..]), TAIL_LINES)
    }
}

/// The last `count` non-blank lines of `text`, oldest first.
pub(crate) fn last_lines(text: &str, count: usize) -> Vec<String> {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let skip = lines.len().saturating_sub(count);
    lines[skip..]
        .iter()
        .map(|line| (*line).to_owned())
        .collect()
}

#[cfg(test)]
#[path = "log_file_test.rs"]
mod log_file_test;
