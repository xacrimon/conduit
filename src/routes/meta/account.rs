use axum::Router;
use axum::routing::get;

use crate::middleware::auth::Session;
use crate::routes::shell;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/meta/account", get(page_account))
}

async fn page_account(session: Session) -> maud::Markup {
    let markup = maud::html! {
        (super::meta_nav("account"))
    };

    shell::document(markup, "account", session)
}
