use crate::AppState;
use crate::auth::Session;
use crate::routes::shell;
use axum::routing::get;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/meta/profile", get(page_profile))
}

async fn page_profile(session: Session) -> maud::Markup {
    let markup = maud::html! {
        (super::meta_nav())
    };

    shell::document(markup, "profile", session)
}
