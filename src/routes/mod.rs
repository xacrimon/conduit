mod assets;
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
use crate::auth::Session;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(assets::routes())
        .merge(autoreload::routes())
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
