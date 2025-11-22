use axum::Router;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::services::{ServeDir, ServeFile};

use crate::AppState;

const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/index.css"));

pub fn routes() -> Router<AppState> {
    Router::new()
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .route("/assets/index.css", get(handle_css))
        .nest_service("/assets", ServeDir::new("public/assets"))
}

async fn handle_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css")], CSS)
}
