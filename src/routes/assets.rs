use crate::AppState;
use tower_http::services::{ServeDir, ServeFile};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .nest_service("/assets", ServeDir::new("public/assets"))
}
