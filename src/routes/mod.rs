mod assets;
#[cfg(debug_assertions)]
mod autoreload;
mod error;
mod hub;
mod meta;
mod paste;
mod shell;

use axum::Router;
use axum::routing::get;
pub use error::AppError;

use crate::AppState;
use crate::middleware::auth::Session;

pub fn routes() -> Router<AppState> {
    let autoreload = cfg_select! {
        debug_assertions => { autoreload::routes() }
        _ => { Router::new() }
    };

    Router::new()
        .merge(assets::routes())
        .merge(autoreload)
        .merge(hub::routes())
        .merge(meta::routes())
        .merge(paste::routes())
        .route("/", get(page))
}

async fn page(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        h1 { "Hello, World!" }
    };

    shell::document(markup, "home", session)
}
