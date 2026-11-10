use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use oc_state::ServerExit;

use super::server_exit;

#[test]
fn exit_statuses_map_to_codes_and_signals() {
    assert_eq!(
        server_exit(ExitStatus::from_raw(1 << 8)),
        ServerExit::Code(1)
    );
    assert_eq!(server_exit(ExitStatus::from_raw(9)), ServerExit::Signal(9));
}
