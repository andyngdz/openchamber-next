//! The tokio runtime that does all I/O, on a thread of its own, so GPUI keeps the main thread.

use std::sync::mpsc as std_mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use futures::channel::oneshot;
use tokio::runtime::{Builder, Handle};

use crate::errors::AppError;

/// How long tasks still running at exit get before the runtime drops them.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);
const THREAD_NAME: &str = "oc-io";

/// A multi-thread runtime owned by one background thread. Dropping it stops the runtime and
/// joins the thread.
#[derive(Debug)]
pub(crate) struct IoRuntime {
    handle: Handle,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl IoRuntime {
    /// Starts the thread and waits until its runtime is built.
    pub(crate) fn start() -> Result<Self, AppError> {
        let (ready, built) = std_mpsc::sync_channel(1);
        let (stop, stopped) = oneshot::channel::<()>();
        let thread = thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || {
                let runtime = match Builder::new_multi_thread().enable_all().build() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        ready.send(Err(error)).ok();
                        return;
                    }
                };
                ready.send(Ok(runtime.handle().clone())).ok();
                runtime.block_on(stopped).ok();
                runtime.shutdown_timeout(SHUTDOWN_GRACE);
            })
            .map_err(AppError::StartRuntime)?;
        let handle = built
            .recv()
            .map_err(|_| AppError::RuntimeThreadExited)?
            .map_err(AppError::StartRuntime)?;
        Ok(Self {
            handle,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    /// Where tasks are spawned.
    pub(crate) fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl Drop for IoRuntime {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.send(()).ok();
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!(target: crate::logging::SERVER, "[SERVER] I/O thread panicked");
        }
    }
}

#[cfg(test)]
#[path = "runtime_test.rs"]
mod runtime_test;
