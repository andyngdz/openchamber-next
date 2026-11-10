use std::sync::mpsc;
use std::time::Duration;

use super::IoRuntime;

#[test]
fn tasks_run_on_the_runtime_and_drop_stops_its_thread() {
    let runtime = IoRuntime::start().expect("runtime starts");
    let (sender, receiver) = mpsc::channel();

    let task = runtime.handle().spawn(async move {
        sender
            .send(std::thread::current().name().map(str::to_owned))
            .ok();
        std::future::pending::<()>().await;
    });
    let worker = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the task ran");
    drop(runtime);

    assert!(worker.is_some_and(|name| name.starts_with("tokio")));
    assert!(task.is_finished(), "a pending task stops with the runtime");
}
