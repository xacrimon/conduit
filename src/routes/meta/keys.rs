use axum::Router;
use axum::routing::get;

use crate::AppState;
use crate::middleware::auth::Session;
use crate::routes::shell;

pub fn routes() -> Router<AppState> {
    Router::new().route("/meta/keys", get(page_keys))
}

async fn page_keys(session: Session) -> maud::Markup {
    let markup = maud::html! {
        (super::meta_nav())
    };

    shell::document(markup, "keys", session)
}
