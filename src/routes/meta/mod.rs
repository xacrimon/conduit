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
    let items = [
        ("profile", "/meta/profile"),
        ("account", "/meta/account"),
        ("keys", "/meta/keys"),
        ("security", "/meta/security"),
    ];

    maud::html! {
        div .border-b .border-gray-300 .mb-3 {
            ul .flex .gap-1 .text-sm {
                @for (name, href) in items {
                    @if name == current {
                        li {
                            a
                                .block
                                .px-2
                                .py-1
                                .bg-gray-200
                                .text-black
                                .border
                                .border-gray-300
                                href=(href)
                            {
                                (name)
                            }
                        }
                    } @else {
                        li {
                            a
                                .block
                                .px-2
                                .py-1
                                .text-gray-600
                                .hover:text-black
                                .hover:bg-gray-100
                                .border
                                .border-transparent
                                href=(href)
                            {
                                (name)
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn meta_redirect() -> Redirect {
    Redirect::to("/meta/profile")
}
