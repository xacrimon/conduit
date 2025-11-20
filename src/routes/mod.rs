mod assets;
mod autoreload;
mod error;
mod meta;
mod paste;

use crate::AppState;
use axum::routing::get;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(assets::routes())
        .merge(autoreload::routes())
        .merge(meta::routes())
        .merge(paste::routes())
        .route("/", get(page))
}

fn document(markup: maud::Markup, title: &str) -> maud::Markup {
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
                (header())
                main { (markup) }
                (footer())
            }
        }
    }
}

fn header() -> maud::Markup {
    maud::html! {}
}

fn footer() -> maud::Markup {
    maud::html! {}
}

async fn page() -> maud::Markup {
    let markup = maud::html! {
        h1 { "Hello, World!" }
    };

    document(markup, "home")
}
