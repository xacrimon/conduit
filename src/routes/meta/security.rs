use axum::Router;
use axum::routing::get;

use crate::middleware::auth::Session;
use crate::routes::shell;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/meta/security", get(page_security))
}

async fn page_security(session: Session) -> maud::Markup {
    let markup = maud::html! {
        (super::meta_nav())
    };

    shell::document(markup, "security", session)
}
