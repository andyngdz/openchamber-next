//! `sidebarProjectSortOrder`: the values React stores, plus the fallback for one this client
//! does not know.
//!
//! React parses the key with `parseOneOf` (`registry.ts:402`), which rejects an unknown value
//! so the key keeps its default, `manual`. An unknown value must not fail the whole settings
//! document, so it decodes to [`SidebarSortOrder::Other`] and orders like `manual`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// How the sidebar orders projects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarSortOrder {
    /// The order the projects array holds.
    #[default]
    Manual,
    /// Label or path, ascending.
    AZ,
    /// Label or path, descending.
    ZA,
    /// Newest `addedAt` first.
    DateAdded,
    /// Newest session activity first.
    Recent,
    /// A value this client does not know; it keeps the stored order, like `manual`.
    Other,
}

/// The stored spelling of every value React knows.
const STORED: [(&str, SidebarSortOrder); 5] = [
    ("manual", SidebarSortOrder::Manual),
    ("a-z", SidebarSortOrder::AZ),
    ("z-a", SidebarSortOrder::ZA),
    ("date-added", SidebarSortOrder::DateAdded),
    ("recent", SidebarSortOrder::Recent),
];

impl<'de> Deserialize<'de> for SidebarSortOrder {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(STORED
            .iter()
            .find(|(name, _)| *name == value)
            .map_or(Self::Other, |(_, order)| *order))
    }
}

impl Serialize for SidebarSortOrder {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let name = STORED
            .iter()
            .find(|(_, order)| order == self)
            .map_or("manual", |(name, _)| *name);
        serializer.serialize_str(name)
    }
}

#[cfg(test)]
#[path = "sort_order_test.rs"]
mod sort_order_test;
