use tokio::select;
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::info;

pub fn bind() -> (CancellationToken, TaskTracker) {
    let token = CancellationToken::new();
    let token_clone = token.clone();
    let tracker = TaskTracker::new();
    let tracker_clone = tracker.clone();

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::spawn(async move {
        select! {
            _ = sigint.recv() => info!("received SIGINT, shutting down"),
            _ = sigterm.recv() => info!("received SIGTERM, shutting down"),
            _ = token_clone.cancelled() => (),
        }

        token_clone.cancel();
        tracker_clone.close();
    });

    (token, tracker)
}
