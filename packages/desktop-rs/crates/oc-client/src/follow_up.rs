//! How the composer sends while a turn runs, read from the shared settings document.
//!
//! `followUpBehavior` replaced the retired `queueModeEnabled` on/off key; both are read here
//! and mapped as `lib/settings/parsers.ts:421-428` does, with `queue` as the default
//! (`stores/messageQueueStore.ts:19`).

use crate::settings::Settings;

/// How a send during a running turn behaves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FollowUpBehavior {
    /// Steer the text into the running turn.
    Steer,
    /// Queue the text in the server's message queue.
    #[default]
    Queue,
}

impl Settings {
    /// The composer's follow-up behaviour: the current key, else the retired on/off key,
    /// else [`FollowUpBehavior::Queue`].
    ///
    /// A stored `immediate` collapses onto `steer`, because on a busy session it was
    /// wire-identical to a steer.
    pub fn follow_up_behavior(&self) -> FollowUpBehavior {
        match self.follow_up_behavior.as_deref() {
            Some("steer" | "immediate") => FollowUpBehavior::Steer,
            Some("queue") => FollowUpBehavior::Queue,
            _ => match self.queue_mode_enabled {
                Some(false) => FollowUpBehavior::Steer,
                Some(true) | None => FollowUpBehavior::Queue,
            },
        }
    }
}

#[cfg(test)]
#[path = "follow_up_test.rs"]
mod follow_up_test;
