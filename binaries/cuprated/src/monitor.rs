use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

pub fn new() -> (CupratedMonitor, CupratedTask) {
    let (error_set, error_watch) = watch::channel(false);
    let task_trackers = TaskTracker::new();
    let cancellation_token = CancellationToken::new();

    (
        CupratedMonitor {
            task_trackers: task_trackers.clone(),
            error_watch,
            cancellation_token: cancellation_token.clone(),
        },
        CupratedTask {
            task_tracker: task_trackers,
            error_set,
            cancellation_token,
        },
    )
}

pub struct CupratedMonitor {
    pub task_trackers: TaskTracker,
    pub error_watch: watch::Receiver<bool>,
    pub cancellation_token: CancellationToken,
}

#[derive(Clone)]
pub struct CupratedTask {
    pub task_tracker: TaskTracker,
    pub error_set: watch::Sender<bool>,
    pub cancellation_token: CancellationToken,
}
