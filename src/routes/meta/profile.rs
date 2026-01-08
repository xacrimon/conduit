use axum::Router;
use axum::routing::get;

use crate::middleware::auth::Session;
use crate::routes::shell;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/meta/profile", get(page_profile))
}

async fn page_profile(session: Session) -> maud::Markup {
    let markup = maud::html! {
        (super::meta_nav())
    };

    shell::document(markup, "profile", session)
}
