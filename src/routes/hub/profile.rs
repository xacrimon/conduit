use axum::Router;
use axum::extract::Path;
use axum::routing::get;

use crate::AppState;
use crate::middleware::auth::Session;
use crate::routes::shell;

pub fn routes() -> Router<AppState> {
    Router::new().route("/~{name}", get(page_profile))
}

async fn page_profile(session: Option<Session>, Path(name): Path<String>) -> maud::Markup {
    let markup = maud::html! {};
    let title = format!("~{}", name);
    shell::document(markup, &title, session)
}
