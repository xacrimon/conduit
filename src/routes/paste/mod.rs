use axum::Router;
use axum::routing::get;

use crate::AppState;
use crate::auth::Session;
use crate::routes::shell;

pub fn routes() -> Router<AppState> {
    Router::new().route("/paste", get(page_paste))
}

async fn page_paste(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        div #editor .relative .w-100 .h-100 { }
        (ace_enable("editor"))
    };

    shell::document_with(markup, "paste", session, ace_script())
}

fn ace_script() -> maud::Markup {
    #[cfg(debug_assertions)]
    maud::html! {
        script defer src="/assets/ace/ace.js" {}
    }
    #[cfg(not(debug_assertions))]
    maud::html! {
        script defer src="/assets/ace-min/ace.js" {}
    }
}

fn ace_enable(id: &str) -> maud::Markup {
    let js = format!(
        r#"
            addEventListener("DOMContentLoaded", (_) => {{
                var editor = ace.edit("{}");
            }})
        "#,
        id
    );

    maud::html! {
        script { (maud::PreEscaped(js)) }
    }
}
