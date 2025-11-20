use crate::AppState;
use axum::response::IntoResponse;
use axum::routing::get;
use std::sync::LazyLock;

static AUTORELOAD_KEY: LazyLock<u64> = LazyLock::new(|| rand::random());

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/autoreload", get(autoreload))
}

async fn autoreload() -> impl IntoResponse {
    AUTORELOAD_KEY.to_string()
}
