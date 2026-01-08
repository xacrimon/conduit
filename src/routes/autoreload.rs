use std::sync::LazyLock;

use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::state::AppState;

static AUTORELOAD_KEY: LazyLock<u64> = LazyLock::new(rand::random);

pub fn routes() -> Router<AppState> {
    Router::new().route("/autoreload", get(autoreload))
}

async fn autoreload() -> impl IntoResponse {
    AUTORELOAD_KEY.to_string()
}
