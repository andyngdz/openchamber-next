//! What one control is, and its reference key.
//!
//! The key follows `tools/capture-reference.mjs`: `<region>/<named ancestors, outermost
//! first>/<name>`, with ` (<role description>)` on a control that holds another control of the
//! same name, and `#<n>` on a key that still repeats in its region. Every key is built from
//! these semantic parts by [`ControlSpec::key`]'s one join, never from a DOM path.

use gpui_kit::SharedString;

use super::registry::{Area, Reveal, Role};

/// One control: region, role, accessible name, where it sits, and its visibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlSpec {
    pub(crate) area: Area,
    pub(crate) role: Role,
    pub(crate) name: SharedString,
    /// The reference key; also the control's `ElementId`.
    pub(crate) key: SharedString,
    pub(crate) reveal: Reveal,
    ancestors: Vec<SharedString>,
    suffix: Option<&'static str>,
    repeat: Option<usize>,
}

impl ControlSpec {
    /// A control directly in its region.
    pub(crate) fn named(area: Area, role: Role, name: impl Into<SharedString>) -> Self {
        let mut spec = Self {
            area,
            role,
            name: name.into(),
            key: SharedString::default(),
            reveal: Reveal::Always,
            ancestors: Vec::new(),
            suffix: None,
            repeat: None,
        };
        spec.key = spec.join();
        spec
    }

    /// Inside these named ancestors, outermost first.
    pub(crate) fn under(mut self, ancestors: &[&str]) -> Self {
        self.ancestors = ancestors
            .iter()
            .map(|name| SharedString::from(name.to_string()))
            .collect();
        self.key = self.join();
        self
    }

    /// A row around a control of the same name: its role description tells the two apart.
    pub(crate) fn described(mut self, description: &'static str) -> Self {
        self.suffix = Some(description);
        self.key = self.join();
        self
    }

    /// The `index`-th (from 1) control whose key would otherwise repeat in its region.
    pub(crate) fn repeated(mut self, index: usize) -> Self {
        self.repeat = Some(index);
        self.key = self.join();
        self
    }

    /// The same control with another visibility.
    pub(crate) fn revealed(mut self, reveal: Reveal) -> Self {
        self.reveal = reveal;
        self
    }

    /// The one place a key is assembled.
    fn join(&self) -> SharedString {
        let mut key = String::from(self.area.json_name());
        for part in self.ancestors.iter().chain(std::iter::once(&self.name)) {
            key.push('/');
            key.push_str(part);
        }
        if let Some(description) = self.suffix {
            key.push_str(" (");
            key.push_str(description);
            key.push(')');
        }
        if let Some(index) = self.repeat {
            key.push('#');
            key.push_str(&index.to_string());
        }
        key.into()
    }
}

#[cfg(test)]
#[path = "control_spec_test.rs"]
mod control_spec_test;
