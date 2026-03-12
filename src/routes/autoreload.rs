use std::convert::Infallible;
use std::time::Duration;

use axum::Router;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures_util::stream::{self, AbortHandle, Abortable, Stream};

use crate::state::AppState;

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub fn routes() -> Router<AppState> {
    Router::new().route("/autoreload", get(autoreload))
}

async fn autoreload(state: AppState) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let ct = state.cancel_token.clone();

    tokio::spawn(async move {
        ct.cancelled().await;
        abort_handle.abort();
    });

    let stream = Abortable::new(stream::pending(), abort_registration);
    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEPALIVE_INTERVAL))
}
