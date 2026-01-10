mod manage;
mod view;

use axum::Router;
use axum::extract::Form;
use axum::response::Redirect;
use axum::routing::{get, post};
use serde::Deserialize;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(view::routes())
        .merge(manage::routes())
        .route("/paste", get(page_paste))
        .route("/paste", post(do_paste))
}

async fn page_paste(session: Session) -> maud::Markup {
    let markup = maud::html! {
        div .mb-4 {
            a .text-blue-600 .hover:underline href="/paste/manage" { "Manage your pastes" }
        }

        h2 .text-xl .mb-4 { "New Paste" }

        form method="post" {
            div .mb-3 {
                label for="filename" .block .mb-1 { "Filename" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .p-2
                    type="text"
                    name="filename"
                    placeholder="example.txt"
                    required;
            }

            div .mb-3 {
                label for="visibility" .block .mb-1 { "Visibility" }
                select
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .p-2
                    name="visibility"
                {
                    option value="public" { "Public - visible to everyone" }
                    option value="unlisted" selected { "Unlisted - only via link" }
                    option value="private" { "Private - only you" }
                }
            }

            div .mb-3 {
                label .block .mb-1 { "Content" }
                input #content_input type="hidden" name="content";
                div #editor .relative .w-full style="height: 400px;" .border-solid .border-1 .border-gray-300 { }
            }

            input
                .text-neutral-50
                .bg-blue-500
                .hover:bg-blue-600
                .border-neutral-700
                .border-solid
                .border-1
                .px-4
                .py-2
                .cursor-pointer
                type="submit"
                value="Create Paste";
        }

        (ace_enable("editor", "content_input"))
    };

    shell::document_with(markup, "new paste", session, ace_script())
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
struct PasteForm {
    filename: String,
    content: String,
    visibility: String,
}

async fn do_paste(
    state: AppState,
    session: Session,
    Form(paste): Form<PasteForm>,
) -> Result<Redirect, AppError> {
    let PasteForm {
        filename,
        content,
        visibility,
    } = paste;

    let filename = if filename.trim().is_empty() {
        "untitled.txt".to_owned()
    } else {
        filename.trim().to_owned()
    };

    let visibility = match visibility.as_str() {
        "public" => model::paste::Visibility::Public,
        "private" => model::paste::Visibility::Private,
        _ => model::paste::Visibility::Unlisted,
    };

    let id =
        model::paste::create_paste(&state.db, session.id, visibility, filename, content).await?;

    let url = format!("/~{}/paste/{}", session.username, id);
    Ok(Redirect::to(&url))
}
