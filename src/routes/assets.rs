use axum::Router;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/index.css"));
pub const CSS_ASSET_NAME: &str = env!("CONDUIT_CSS_ASSET_NAME");

pub fn routes() -> Router<AppState> {
    let css_route = format!("/assets/{}", CSS_ASSET_NAME);
    Router::new()
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .route(&css_route, get(handle_css))
        .nest_service("/assets", ServeDir::new("public/assets"))
}

async fn handle_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css")], CSS)
}
