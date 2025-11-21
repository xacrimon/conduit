mod keys;
mod login;
mod logout;
mod profile;
mod register;

use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(login::routes())
        .merge(logout::routes())
        .merge(register::routes())
        .merge(profile::routes())
        .merge(keys::routes())
        .route("/meta", get(meta_redirect))
}

fn meta_nav() -> maud::Markup {
    maud::html! {
        div .container {
            div {
                ul .nav {
                    li { a href="/meta/profile" { "profile" } }
                    li { a href="/meta/keys" { "keys" } }
                }
            }
        }
    }
}

async fn meta_redirect() -> Redirect {
    Redirect::to("/meta/profile")
}
