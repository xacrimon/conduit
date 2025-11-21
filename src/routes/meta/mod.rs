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
        div {
            ul .flex .gap-4 {
                li { a .hover:underline href="/meta/profile" { "profile" } }
                li { a .hover:underline href="/meta/keys" { "keys" } }
            }
        }
    }
}

async fn meta_redirect() -> Redirect {
    Redirect::to("/meta/profile")
}
