use axum::extract::Path;
use axum::routing::get;

use crate::AppState;
use crate::auth::Session;
use crate::routes::shell;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/~{name}", get(page_profile))
}

async fn page_profile(session: Option<Session>, Path(name): Path<String>) -> maud::Markup {
    let markup = maud::html! {};
    let title = format!("~{}", name);
    shell::document(markup, &title, session)
}
