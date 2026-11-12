use oc_client::routing::PermissionMode;

use crate::permission_mode::{SafetyNet, displayed_permission_mode, next_permission_mode};

#[test]
fn displays_safety_as_ask_when_the_net_is_unavailable() {
    assert_eq!(
        displayed_permission_mode(PermissionMode::Safety, SafetyNet::Unavailable),
        PermissionMode::Ask
    );
    assert_eq!(
        displayed_permission_mode(PermissionMode::Safety, SafetyNet::Available),
        PermissionMode::Safety
    );
}

#[test]
fn cycles_ask_safety_auto_while_the_net_is_available() {
    assert_eq!(
        next_permission_mode(PermissionMode::Ask, SafetyNet::Available),
        PermissionMode::Safety
    );
    assert_eq!(
        next_permission_mode(PermissionMode::Safety, SafetyNet::Available),
        PermissionMode::Auto
    );
    assert_eq!(
        next_permission_mode(PermissionMode::Auto, SafetyNet::Available),
        PermissionMode::Ask
    );
}

#[test]
fn skips_safety_while_it_is_unavailable() {
    assert_eq!(
        next_permission_mode(PermissionMode::Ask, SafetyNet::Unavailable),
        PermissionMode::Auto
    );
    assert_eq!(
        next_permission_mode(PermissionMode::Safety, SafetyNet::Unavailable),
        PermissionMode::Auto
    );
    assert_eq!(
        next_permission_mode(PermissionMode::Auto, SafetyNet::Unavailable),
        PermissionMode::Ask
    );
}
