use std::fs;
use std::path::Path;

use nix::unistd::Pid;

const PROC_ROOT: &str = "/proc";

/// Pids of `opencode serve` processes whose parent is `parent`, read from
/// `/proc`. OpenCode runs in its own process group but keeps the CLI as its
/// parent (`packages/web/server/lib/opencode/lifecycle.js:428-434`). A process
/// that exits during the scan is skipped.
pub(crate) fn opencode_children(parent: Pid) -> Vec<RecordedPid> {
    let Ok(entries) = fs::read_dir(PROC_ROOT) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str()?.parse::<i32>().ok())
        .map(Pid::from_raw)
        .filter(|pid| parent_of(*pid) == Some(parent) && is_opencode_serve_pid(*pid))
        .filter_map(RecordedPid::capture)
        .collect()
}

/// An `opencode serve` process seen at one moment: its pid and the clock tick
/// it started at, which tells it apart from a later process that reuses the pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecordedPid {
    pub(crate) pid: Pid,
    pub(crate) start_time: u64,
}

impl RecordedPid {
    /// Reads the start time of `pid`; `None` when the process is gone.
    pub(crate) fn capture(pid: Pid) -> Option<Self> {
        Some(Self {
            pid,
            start_time: start_time_of(pid)?,
        })
    }

    /// Whether this is still the same `opencode serve` process: the command
    /// line matches and the start time is unchanged, so a pid the kernel
    /// reused for anything else is never signalled.
    pub(crate) fn is_still_running(&self) -> bool {
        is_opencode_serve_pid(self.pid) && start_time_of(self.pid) == Some(self.start_time)
    }
}

/// Whether `pid` runs `opencode serve` right now.
pub(crate) fn is_opencode_serve_pid(pid: Pid) -> bool {
    fs::read(proc_file(pid, "cmdline")).is_ok_and(|cmdline| is_opencode_serve(&cmdline))
}

fn read_stat(pid: Pid) -> Option<String> {
    fs::read_to_string(proc_file(pid, "stat")).ok()
}

fn start_time_of(pid: Pid) -> Option<u64> {
    parse_stat_start_time(&read_stat(pid)?)
}

fn parent_of(pid: Pid) -> Option<Pid> {
    parse_stat_ppid(&read_stat(pid)?).map(Pid::from_raw)
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

/// The start time (field 22, in clock ticks since boot) from a
/// `/proc/<pid>/stat` line; the fields after the last `)` begin at field 3.
pub(crate) fn parse_stat_start_time(stat: &str) -> Option<u64> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    after_comm.split_whitespace().nth(19)?.parse().ok()
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
