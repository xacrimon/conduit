use crate::AppState;
use crate::auth::Session;
use crate::routes::document;
use axum::routing::get;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/~{username}", get(page_profile))
}

async fn page_profile(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {};
    document(markup, "profile", session)
}
