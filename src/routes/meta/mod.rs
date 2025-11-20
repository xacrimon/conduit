mod login;
mod logout;
mod profile;
mod register;

use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::auth::Session;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(login::routes())
        .merge(logout::routes())
        .merge(register::routes())
        .merge(profile::routes())
        .route("/meta", get(meta_redirect))
}

fn document<S: Into<Option<Session>>>(
    markup: maud::Markup,
    title: &str,
    session: S,
) -> maud::Markup {
    super::document(markup, title, session)
}

async fn meta_redirect() -> Redirect {
    Redirect::to("/meta/profile")
}
