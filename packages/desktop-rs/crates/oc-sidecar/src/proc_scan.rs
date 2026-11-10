use std::fs;
use std::path::Path;

use nix::unistd::Pid;

const PROC_ROOT: &str = "/proc";

/// Pids of `opencode serve` processes whose parent is `parent`, read from
/// `/proc`. OpenCode runs in its own process group but keeps the CLI as its
/// parent (`packages/web/server/lib/opencode/lifecycle.js:428-434`). A process
/// that exits during the scan is skipped.
pub(crate) fn opencode_children(parent: Pid) -> Vec<Pid> {
    let Ok(entries) = fs::read_dir(PROC_ROOT) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str()?.parse::<i32>().ok())
        .map(Pid::from_raw)
        .filter(|pid| parent_of(*pid) == Some(parent) && is_opencode_serve_pid(*pid))
        .collect()
}

/// Whether `pid` is still an `opencode serve` process, so a pid the kernel
/// reused for something else is never signalled.
pub(crate) fn is_opencode_serve_pid(pid: Pid) -> bool {
    fs::read(proc_file(pid, "cmdline")).is_ok_and(|cmdline| is_opencode_serve(&cmdline))
}

fn parent_of(pid: Pid) -> Option<Pid> {
    let stat = fs::read_to_string(proc_file(pid, "stat")).ok()?;
    parse_stat_ppid(&stat).map(Pid::from_raw)
}

fn proc_file(pid: Pid, name: &str) -> std::path::PathBuf {
    Path::new(PROC_ROOT).join(pid.to_string()).join(name)
}

/// The parent pid from a `/proc/<pid>/stat` line. The command name sits in
/// parentheses and may hold spaces or parentheses itself, so the fields after
/// the last `)` are read: state, then ppid.
pub(crate) fn parse_stat_ppid(stat: &str) -> Option<i32> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    after_comm.split_whitespace().nth(1)?.parse().ok()
}

/// Whether a NUL-separated `/proc/<pid>/cmdline` runs `opencode serve`: one
/// argument mentions `opencode` and another is exactly `serve`.
pub(crate) fn is_opencode_serve(cmdline: &[u8]) -> bool {
    let args: Vec<&[u8]> = cmdline
        .split(|byte| *byte == 0)
        .filter(|arg| !arg.is_empty())
        .collect();
    let names_opencode = args.iter().any(|arg| {
        arg.windows(b"opencode".len())
            .any(|window| window == b"opencode")
    });
    names_opencode && args.contains(&b"serve".as_slice())
}

#[cfg(test)]
#[path = "proc_scan_test.rs"]
mod proc_scan_test;
