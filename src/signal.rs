use tokio::select;
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

async fn stop_signal() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    select! {
        _ = sigint.recv() => (),
        _ = sigterm.recv() => (),
    }
}

pub fn bind() -> (CancellationToken, TaskTracker) {
    let token = CancellationToken::new();
    let token_clone = token.clone();
    let tracker = TaskTracker::new();
    let tracker_clone = tracker.clone();

    tokio::spawn(async move {
        select! {
            _ = stop_signal() => (),
            _ = token_clone.cancelled() => (),
        }

        token_clone.cancel();
        tracker_clone.close();
    });

    (token, tracker)
}
