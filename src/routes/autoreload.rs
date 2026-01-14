use std::convert::Infallible;
use std::time::Duration;

use axum::Router;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures_util::stream::{self, Stream};

use crate::state::AppState;

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub fn routes() -> Router<AppState> {
    Router::new().route("/autoreload", get(autoreload))
}

async fn autoreload() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::pending();
    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEPALIVE_INTERVAL))
}
