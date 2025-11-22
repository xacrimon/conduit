use axum::Router;
use axum::extract::Form;
use axum::response::Redirect;
use axum::routing::{get, post};
use serde::Deserialize;

use crate::AppState;
use crate::auth::Session;
use crate::routes::{AppError, shell};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/paste", get(page_paste))
        .route("/paste", post(do_paste))
}

async fn page_paste(session: Session) -> maud::Markup {
    let markup = maud::html! {
        form method="post" {
            input type="text" name="filename" placeholder="file name";
            input #content_input type="hidden" name="content";
            div #editor .relative .w-100 .h-100 .border-solid .border-1 { }
            input type="submit" value="create paste";
        }

        (ace_enable("editor", "content_input"))
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

fn ace_enable(editor_id: &str, input_id: &str) -> maud::Markup {
    let js = format!(
        r#"
            addEventListener("DOMContentLoaded", (_) => {{
                let editor = ace.edit("{}");
                let input = document.getElementById("{}");
                editor.on("change", () => input.value = editor.getValue());
            }})
        "#,
        editor_id, input_id,
    );

    maud::html! {
        script { (maud::PreEscaped(js)) }
    }
}

#[derive(Deserialize)]
struct Paste {
    filename: String,
    content: String,
}

async fn do_paste(session: Session, Form(paste): Form<Paste>) -> Result<Redirect, AppError> {
    let Paste { filename, content } = paste;
    let filename = if filename.is_empty() {
        None
    } else {
        Some(filename)
    };

    Ok(Redirect::to("/"))
}
