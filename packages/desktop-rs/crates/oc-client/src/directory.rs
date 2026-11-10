//! The absolute directory a scoped OpenCode call runs in.

use std::fmt;

/// An absolute path the server records a session under.
///
/// The client keeps no directory of its own: every scoped call takes one, so several
/// projects can share a client and one global event stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Directory(String);

impl Directory {
    /// The directory as the server records it. Callers canonicalize a user path before
    /// creating this; the server lists a session only under the path it was created with.
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The path as sent in the `x-opencode-directory` header and the `directory` query.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Directory {
    fn from(path: String) -> Self {
        Self(path)
    }
}

impl From<&str> for Directory {
    fn from(path: &str) -> Self {
        Self(path.to_owned())
    }
}

impl fmt::Display for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "directory_test.rs"]
mod directory_test;
