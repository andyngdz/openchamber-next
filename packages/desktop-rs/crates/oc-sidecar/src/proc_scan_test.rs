use nix::unistd::{Pid, getpid};

use super::{
    is_opencode_serve, is_opencode_serve_pid, opencode_children, parse_stat_ppid,
    parse_stat_start_time,
};

/// A `/proc/<pid>/cmdline` value: every argument followed by a NUL byte.
fn cmdline(args: &[&str]) -> Vec<u8> {
    args.iter()
        .flat_map(|arg| arg.bytes().chain(std::iter::once(0)))
        .collect()
}

#[test]
fn reads_the_ppid_after_the_command_name() {
    assert_eq!(parse_stat_ppid("1234 (node) S 99 1234 1234 0"), Some(99));
    assert_eq!(parse_stat_ppid("7 (a (b) c) R 42 7 7"), Some(42));
    assert_eq!(parse_stat_ppid("garbage"), None);
}

#[test]
fn reads_the_start_time_after_the_command_name() {
    let stat = "7 (a (b) c) S 42 7 7 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 1 0 987654 1000 50";
    assert_eq!(parse_stat_start_time(stat), Some(987_654));
    assert_eq!(parse_stat_start_time("7 (x) S 42"), None);
    assert_eq!(parse_stat_start_time("garbage"), None);
}

#[test]
fn recognises_opencode_serve_command_lines() {
    assert!(is_opencode_serve(&cmdline(&[
        "/home/u/.opencode/bin/opencode",
        "serve",
        "--hostname",
        "127.0.0.1",
        "--port",
        "0",
    ])));
    assert!(is_opencode_serve(&cmdline(&[
        "node",
        "/usr/lib/opencode-ai/bin/opencode",
        "serve",
    ])));
}

#[test]
fn ignores_other_command_lines() {
    assert!(!is_opencode_serve(&cmdline(&[
        "node",
        "cli.js",
        "serve",
        "--foreground"
    ])));
    assert!(!is_opencode_serve(&cmdline(&["opencode", "run"])));
    assert!(!is_opencode_serve(&cmdline(&["vim", "opencode-serve.txt"])));
    assert!(!is_opencode_serve(b""));
}

#[test]
fn this_test_process_has_no_opencode_children() {
    assert!(opencode_children(getpid()).is_empty());
    assert!(!is_opencode_serve_pid(getpid()));
    assert!(!is_opencode_serve_pid(Pid::from_raw(i32::MAX)));
}
