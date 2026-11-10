//! The per-character match mask Fuse builds while it scans.
//!
//! Fuse allocates `Array(textLen)`, lets assignments grow it past the text when the bit scan
//! reaches further, and turns it into index pairs only to test whether any run is at least
//! `minMatchCharLength` long (`includeMatches` is false, so the pairs themselves are unused).

/// The characters Fuse records as matched.
pub(super) struct MatchMask {
    units: Vec<u8>,
}

impl MatchMask {
    /// A mask with one empty unit per code unit of the text.
    pub(super) fn new(length: usize) -> Self {
        Self {
            units: vec![0; length],
        }
    }

    /// Marks `index`, growing the mask as a JavaScript array assignment does.
    pub(super) fn set(&mut self, index: usize, value: u8) {
        if index >= self.units.len() {
            self.units.resize(index + 1, 0);
        }
        self.units[index] = value;
    }

    /// Whether `convertMaskToIndices(mask, minimum_length)` would find a run at least that long.
    pub(super) fn has_run(&self, minimum_length: usize) -> bool {
        let mut start: Option<usize> = None;
        for (index, value) in self.units.iter().enumerate() {
            if *value != 0 {
                if start.is_none() {
                    start = Some(index);
                }
            } else if let Some(begin) = start.take()
                && index - begin >= minimum_length
            {
                return true;
            }
        }
        start.is_some_and(|begin| self.units.len() - begin >= minimum_length)
    }
}

#[cfg(test)]
#[path = "mask_test.rs"]
mod mask_test;
