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

fn meta_nav() -> maud::Markup {
    maud::html! {
        div {
            ul .flex .gap-4 {
                li { a .hover:underline href="/meta/profile" { "profile" } }
                li { a .hover:underline href="/meta/account" { "account" } }
                li { a .hover:underline href="/meta/keys" { "keys" } }
                li { a .hover:underline href="/meta/security" { "security" } }
            }
        }
    }
}

async fn meta_redirect() -> Redirect {
    Redirect::to("/meta/profile")
}
