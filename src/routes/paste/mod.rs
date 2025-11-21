use crate::AppState;
use crate::auth::Session;
use crate::routes::shell;
use axum::routing::get;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/paste", get(page_paste))
}

async fn page_paste(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {};
    shell::document(markup, "paste", session)
}
