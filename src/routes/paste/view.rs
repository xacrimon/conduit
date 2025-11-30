use axum::Router;
use axum::extract::Path;
use axum::routing::get;

use crate::AppState;
use crate::middleware::auth::Session;
use crate::routes::shell;

pub fn routes() -> Router<AppState> {
    Router::new().route("/~{name}/paste/{id}", get(page_view_paste))
}

async fn page_view_paste(
    Path((username, id)): Path<(String, String)>,
    session: Option<Session>,
) -> maud::Markup {
    let markup = maud::html! {};

    let title = format!("paste {id}");
    shell::document(markup, &title, session)
}
