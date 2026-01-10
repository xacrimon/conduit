mod assets;
#[cfg(debug_assertions)]
mod autoreload;
mod error;
mod hub;
mod lfs;
mod login;
mod meta;
mod paste;
mod shell;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
pub use error::AppError;

use crate::middleware::auth::Session;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    let autoreload = cfg_select! {
        debug_assertions => { autoreload::routes() }
        _ => { Router::new() }
    };

    Router::new()
        .merge(assets::routes())
        .merge(autoreload)
        .merge(login::routes())
        .merge(hub::routes())
        .merge(lfs::routes())
        .merge(meta::routes())
        .merge(paste::routes())
        .route("/", get(page))
        .fallback(fallback)
}

async fn page(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        h1 { "Hello, World!" }
    };

    shell::document(markup, "home", session)
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}
