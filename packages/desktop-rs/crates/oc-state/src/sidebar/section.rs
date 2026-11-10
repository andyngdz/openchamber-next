//! The built-in sidebar sections, in the stored order and behind their settings gates.
//!
//! `sidebarSections` is made whole by `sidebarSectionsSchema`
//! (`packages/ui/src/lib/sidebarSections.ts:158-196`): repeated or unknown entries are dropped
//! and a built-in section missing from the stored list goes back where the default order puts
//! it. `resolveVisibleSidebarSections` then reads the show toggles and the hidden list
//! (`packages/ui/src/components/session/sidebar/sections/sectionMembership.ts:27-64`).

use oc_client::settings::{Settings, SidebarSectionKind};

/// The four sections every client has (`sidebarSections.ts:17`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltinSection {
    /// Managed Chat sessions.
    Chats,
    /// Pins filed in the Pinned list.
    Pinned,
    /// Sessions whose work state is open.
    Work,
    /// One group per project.
    Projects,
}

impl BuiltinSection {
    /// The default order (`sidebarSections.ts:163`).
    pub(crate) const ALL: [Self; 4] = [Self::Chats, Self::Pinned, Self::Work, Self::Projects];

    /// The stored id.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Chats => "chats",
            Self::Pinned => "pinned",
            Self::Work => "work",
            Self::Projects => "projects",
        }
    }

    /// The stored id, if this is one of the four built-ins.
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|section| section.id() == id)
    }
}

/// The sections this surface draws, in the saved order.
pub(crate) fn visible(settings: &Settings) -> Vec<BuiltinSection> {
    let mut entries = stored_order(settings);
    add_missing(&mut entries);
    entries
        .into_iter()
        .filter(|section| is_shown(*section, settings))
        .collect()
}

/// The stored built-ins, first occurrence only; custom and unknown entries are left out.
fn stored_order(settings: &Settings) -> Vec<BuiltinSection> {
    let mut entries: Vec<BuiltinSection> = Vec::new();
    for entry in &settings.sidebar_sections {
        if entry.kind != SidebarSectionKind::Builtin {
            continue;
        }
        let Some(section) = BuiltinSection::from_id(&entry.id) else {
            continue;
        };
        if !entries.contains(&section) {
            entries.push(section);
        }
    }
    entries
}

/// Puts every missing built-in after the built-in that precedes it by default
/// (`sidebarSections.ts:187-194`).
fn add_missing(entries: &mut Vec<BuiltinSection>) {
    for (index, section) in BuiltinSection::ALL.into_iter().enumerate() {
        if entries.contains(&section) {
            continue;
        }
        let after = BuiltinSection::ALL[..index]
            .iter()
            .rev()
            .find_map(|previous| entries.iter().position(|entry| entry == previous));
        let at = after.map_or(0, |position| position + 1);
        entries.insert(at, section);
    }
}

/// The show toggles and the surface's hidden list (`sectionMembership.ts:51-63`).
fn is_shown(section: BuiltinSection, settings: &Settings) -> bool {
    let shown = match section {
        BuiltinSection::Chats => settings.sidebar_show_chats_section,
        BuiltinSection::Pinned => settings.sidebar_show_pinned_section,
        BuiltinSection::Work => settings.session_work_enabled && settings.sidebar_show_work_section,
        BuiltinSection::Projects => true,
    };
    shown
        && !settings
            .sidebar_hidden_sections
            .iter()
            .any(|hidden| hidden == section.id())
}

#[cfg(test)]
#[path = "section_test.rs"]
mod section_test;
