use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read};
use std::process::{ChildStderr, ChildStdout};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

use crate::errors::SidecarError;
use crate::log_file::TAIL_LINES;

/// What the stdout reader saw first.
pub(crate) enum FirstLine {
    Line(String),
    Closed,
    Failed(io::Error),
}

/// Sends the first stdout line, then drains the pipe so the CLI never blocks
/// on a full pipe. The reader thread ends when the pipe closes.
pub(crate) fn follow_stdout(
    stdout: Option<ChildStdout>,
) -> Result<mpsc::Receiver<FirstLine>, SidecarError> {
    let (sender, receiver) = mpsc::sync_channel(1);
    let Some(stdout) = stdout else {
        sender.send(FirstLine::Closed).ok();
        return Ok(receiver);
    };
    spawn_reader("oc-sidecar-stdout", move || {
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        let first = match reader.read_until(b'\n', &mut line) {
            Ok(0) => FirstLine::Closed,
            Ok(_) => FirstLine::Line(String::from_utf8_lossy(&line).into_owned()),
            Err(error) => FirstLine::Failed(error),
        };
        sender.send(first).ok();
        io::copy(&mut reader, &mut io::sink()).ok();
    })?;
    Ok(receiver)
}

/// The last stderr lines of the CLI, collected by a reader thread that ends
/// when the pipe closes.
pub(crate) struct StderrTail {
    lines: Arc<Mutex<VecDeque<String>>>,
    reader: Option<JoinHandle<()>>,
}

impl StderrTail {
    pub(crate) fn follow(stderr: Option<ChildStderr>) -> Result<Self, SidecarError> {
        let lines = Arc::new(Mutex::new(VecDeque::with_capacity(TAIL_LINES)));
        let Some(stderr) = stderr else {
            return Ok(Self {
                lines,
                reader: None,
            });
        };
        let shared = Arc::clone(&lines);
        let reader = spawn_reader("oc-sidecar-stderr", move || {
            collect_tail(stderr, &shared);
        })?;
        Ok(Self {
            lines,
            reader: Some(reader),
        })
    }

    /// Waits until stderr is closed, so the snapshot holds the CLI's last
    /// words. Only called after the CLI exited; OpenCode has pipes of its own.
    pub(crate) fn wait_for_close(&mut self) {
        if let Some(reader) = self.reader.take() {
            reader.join().ok();
        }
    }

    pub(crate) fn snapshot(&self) -> Vec<String> {
        let lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        lines.iter().cloned().collect()
    }
}

fn collect_tail(stderr: impl Read, lines: &Mutex<VecDeque<String>>) {
    for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
        let text = String::from_utf8_lossy(&line).trim_end().to_owned();
        if text.is_empty() {
            continue;
        }
        let mut tail = lines.lock().unwrap_or_else(PoisonError::into_inner);
        if tail.len() == TAIL_LINES {
            tail.pop_front();
        }
        tail.push_back(text);
    }
}

fn spawn_reader<F: FnOnce() + Send + 'static>(
    name: &str,
    body: F,
) -> Result<JoinHandle<()>, SidecarError> {
    thread::Builder::new()
        .name(name.to_owned())
        .spawn(body)
        .map_err(|source| SidecarError::Spawn {
            program: name.to_owned(),
            source,
        })
}

#[cfg(test)]
#[path = "pipes_test.rs"]
mod pipes_test;
