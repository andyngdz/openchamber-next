//! Filesystem answers the app reads: where the user's home and Chat folders live, and the
//! directory the app creates for a Chat session.

use serde::{Deserialize, Serialize};

/// `GET /api/fs/home`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Home {
    /// The user's home directory.
    pub home: String,
    /// Where a new Chat folder goes; older servers omit it.
    #[serde(rename = "chatsRoot", default)]
    pub chats_root: String,
    /// The configured Chats root with symlinks resolved.
    #[serde(rename = "canonicalChatsRoot", default)]
    pub canonical_chats_root: Option<String>,
    /// The legacy Chats root with symlinks resolved.
    #[serde(rename = "canonicalLegacyChatsRoot", default)]
    pub canonical_legacy_chats_root: Option<String>,
}

/// Body of `POST /api/fs/mkdir`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MkdirRequest {
    /// The folder to create, at least one level under an allowed root.
    pub path: String,
}

/// Answer of `POST /api/fs/mkdir`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MkdirResult {
    /// Whether the folder exists after the call.
    pub success: bool,
    /// The folder the server created.
    #[serde(default)]
    pub path: String,
}

#[cfg(test)]
#[path = "filesystem_test.rs"]
mod filesystem_test;
