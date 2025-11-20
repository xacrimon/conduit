use crate::AppState;
use crate::auth::Session;
use crate::routes::document;
use axum::routing::get;
use axum::extract::Path;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/~{name}", get(page_profile))
}

async fn page_profile(session: Option<Session>, Path(name): Path<String>) -> maud::Markup {
    let markup = maud::html! {};

    let title = format!("~{}", name);
    document(markup, &title, session)
}
