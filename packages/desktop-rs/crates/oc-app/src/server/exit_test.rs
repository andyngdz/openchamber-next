use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use oc_state::ServerExit;

use super::{Liveness, liveness, liveness_of_stat, server_exit};

#[test]
fn a_zombie_or_dead_stat_has_exited() {
    assert_eq!(
        liveness_of_stat("4242 (node) Z 1 4242 4242 0"),
        Liveness::Exited
    );
    assert_eq!(liveness_of_stat("4242 (node) X 1 4242"), Liveness::Exited);
}

#[test]
fn a_running_or_sleeping_stat_is_running() {
    assert_eq!(liveness_of_stat("4242 (node) S 1 4242"), Liveness::Running);
    assert_eq!(liveness_of_stat("4242 (node) R 1 4242"), Liveness::Running);
    assert_eq!(liveness_of_stat("4242 (no) Z (de)) S 1"), Liveness::Running);
    assert_eq!(liveness_of_stat(""), Liveness::Running);
}

#[test]
fn this_process_is_running() {
    let pid = i32::try_from(std::process::id()).expect("pid fits");
    assert_eq!(liveness(pid), Liveness::Running);
    assert_eq!(liveness(i32::MAX), Liveness::Exited, "no such pid");
}

#[test]
fn exit_statuses_map_to_codes_and_signals() {
    assert_eq!(
        server_exit(ExitStatus::from_raw(1 << 8)),
        ServerExit::Code(1)
    );
    assert_eq!(server_exit(ExitStatus::from_raw(9)), ServerExit::Signal(9));
}
