mod keys;
mod profile;

use axum::Router;
use axum::response::Redirect;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
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
