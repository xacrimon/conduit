mod view;

use axum::Router;
use axum::extract::{Form, State};
use axum::response::Redirect;
use axum::routing::{get, post};
use serde::Deserialize;

use crate::auth::Session;
use crate::routes::{AppError, shell};
use crate::{AppState, model};

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(view::routes())
        .route("/paste", get(page_paste))
        .route("/paste", post(do_paste))
}

async fn page_paste(session: Session) -> maud::Markup {
    let markup = maud::html! {
        form method="post" {
            input .border-solid .border-1 type="text" name="filename" placeholder="file name";
            input #content_input type="hidden" name="content";
            div #editor .relative .w-100 .h-100 .border-solid .border-1 { }
            input .text-neutral-50 .bg-blue-500 .border-neutral-700 .border-solid .border-1 type="submit" value="create paste";
        }

        (ace_enable("editor", "content_input"))
    };

    shell::document_with(markup, "paste", session, ace_script())
}

fn ace_script() -> maud::Markup {
    maud::html! {
        script defer src="/assets/ace/ace.js" {}
    }
}

fn ace_enable(editor_id: &str, input_id: &str) -> maud::Markup {
    let js = format!(
        r#"
            addEventListener("DOMContentLoaded", (_) => {{
                let editor = ace.edit("{editor_id}");
                let input = document.getElementById("{input_id}");
                editor.on("change", () => input.value = editor.getValue());
            }})
        "#,
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

async fn do_paste(
    State(state): State<AppState>,
    session: Session,
    Form(paste): Form<Paste>,
) -> Result<Redirect, AppError> {
    let Paste { filename, content } = paste;
    let filename = if filename.is_empty() {
        "default.txt".to_owned()
    } else {
        filename
    };

    let id = model::paste::create_paste(
        &state.db,
        session.id,
        model::paste::Visibility::Public,
        filename,
        content,
    )
    .await?;

    let url = format!("/~{}/paste/{}", session.username, id);
    Ok(Redirect::to(&url))
}
