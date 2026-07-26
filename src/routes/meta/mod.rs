mod account;
mod keys;
mod profile;
mod security;

use axum::Router;
use axum::response::Redirect;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(profile::routes())
        .merge(keys::routes())
        .merge(account::routes())
        .merge(security::routes())
        .route("/meta", get(meta_redirect))
}

fn meta_nav(current: &str) -> maud::Markup {
    super::shell::subnav(
        &[
            ("profile", "/meta/profile"),
            ("account", "/meta/account"),
            ("keys", "/meta/keys"),
            ("security", "/meta/security"),
        ],
        current,
    )
}

async fn meta_redirect() -> Redirect {
    Redirect::to("/meta/profile")
}
