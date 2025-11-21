use crate::AppState;
use crate::auth::Session;
use crate::routes::document;
use axum::routing::get;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/meta/keys", get(page_keys))
}

async fn page_keys(session: Session) -> maud::Markup {
    let markup = maud::html! {
        (super::meta_nav())
    };

    document(markup, "keys", session)
}
