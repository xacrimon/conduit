mod assets;
mod autoreload;
mod error;
mod meta;
mod paste;

pub use error::AppError;

use crate::AppState;
use crate::auth::Session;
use axum::routing::get;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(assets::routes())
        .merge(autoreload::routes())
        .merge(meta::routes())
        .merge(paste::routes())
        .route("/", get(page))
}

fn document(markup: maud::Markup, title: &str, session: Option<Session>) -> maud::Markup {
    maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" href="/assets/reset.css";
                link rel="stylesheet" href="/assets/index.css";
                script src="/assets/htmx-2.0.8.js" {}
                script src="/assets/autoreload.js" {}
                title { (title) " - conduit" }
            }

            body {
                (header(&session))
                main { (markup) }
            }
        }
    }
}

fn header(session: &Option<Session>) -> maud::Markup {
    maud::html! {
        nav {
            span {
                a href="/" { "conduit" }
            }
            @if let Some(session) = session {
                ul {
                    li {
                        a href="/paste" { "paste" }
                    }
                }

                div {
                    span {
                        "Logged in as "
                        a href={"/~" (session.username)} { (session.username) }
                        " - "
                        a href="/logout" { "Log out" }
                    }
                }
            } @else {
                div {
                    span {
                        a href="/login" { "Log in" }
                        " - "
                        a href="/register" { "Register" }
                    }
                }
            }
        }
    }
}

async fn page(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        h1 { "Hello, World!" }
    };

    document(markup, "home", session)
}
