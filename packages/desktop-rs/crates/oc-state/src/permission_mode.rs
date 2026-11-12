//! The shield's displayed mode and click cycle (`stores/utils/permissionAutoAccept.ts:22-32`).

use oc_client::routing::PermissionMode;

/// Whether the safety net can classify an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyNet {
    /// `available && jevAvailable` (`useRoutingStore.ts:160`).
    Available,
    /// Routing is off, or no classification provider is configured.
    Unavailable,
}

impl SafetyNet {
    /// The boolean the routing state answers with.
    pub fn available(self) -> bool {
        self == Self::Available
    }
}

/// The mode the user sees; a `safety` session without a safety net waits like `ask`.
pub fn displayed_permission_mode(mode: PermissionMode, net: SafetyNet) -> PermissionMode {
    match (mode, net) {
        (PermissionMode::Safety, SafetyNet::Unavailable) => PermissionMode::Ask,
        (mode, _) => mode,
    }
}

/// The composer button's cycle: ask → safety → auto → ask, skipping safety when unavailable.
pub fn next_permission_mode(mode: PermissionMode, net: SafetyNet) -> PermissionMode {
    match (displayed_permission_mode(mode, net), net) {
        (PermissionMode::Ask, SafetyNet::Available) => PermissionMode::Safety,
        (PermissionMode::Ask | PermissionMode::Safety, SafetyNet::Unavailable) => {
            PermissionMode::Auto
        }
        (PermissionMode::Safety, SafetyNet::Available) => PermissionMode::Auto,
        (PermissionMode::Auto, _) => PermissionMode::Ask,
    }
}

#[cfg(test)]
#[path = "permission_mode_test.rs"]
mod permission_mode_test;
