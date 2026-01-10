use axum::Router;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/~{username}/paste/{id}", get(page_view_paste))
}

async fn page_view_paste(
    state: AppState,
    Path((_username, id)): Path<(String, String)>,
    session: Option<Session>,
) -> Result<Response, AppError> {
    let paste = model::paste::get_paste(&state.db, &id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Paste not found"))?;

    if paste.visibility == "private" {
        let is_owner = session.as_ref().is_some_and(|s| s.id == paste.user_id);
        if !is_owner {
            return Err(anyhow::anyhow!("Paste not found").into());
        }
    }

    // Infer language mode from filename extension
    let mode = infer_ace_mode(&paste.filename);

    let markup = maud::html! {
        div .mb-4 {
            h2 .text-xl .font-mono { (paste.filename) }
            div .text-sm .text-gray-600 .mt-1 {
                span .mr-3 { "ID: " (paste.id) }
                @if paste.visibility == "public" {
                    span .text-xs .bg-green-100 .text-green-800 .px-2 .py-1 .rounded { "public" }
                } @else if paste.visibility == "unlisted" {
                    span .text-xs .bg-yellow-100 .text-yellow-800 .px-2 .py-1 .rounded { "unlisted" }
                } @else {
                    span .text-xs .bg-gray-100 .text-gray-800 .px-2 .py-1 .rounded { "private" }
                }
            }
        }

        div #editor .relative .w-full style="height: 600px;" .border-solid .border-1 .border-gray-300 {
            (paste.content)
        }

        (ace_readonly("editor", mode))
    };

    let title = format!("{} - paste", paste.filename);
    Ok(shell::document_with(markup, &title, session, ace_script()).into_response())
}

fn ace_script() -> maud::Markup {
    maud::html! {
        script defer src="/assets/ace/ace.js" {}
    }
}

fn ace_readonly(editor_id: &str, mode: &str) -> maud::Markup {
    let js = format!(
        r#"
            addEventListener("DOMContentLoaded", (_) => {{
                let editor = ace.edit("{editor_id}");
                editor.setTheme("ace/theme/github");
                editor.session.setMode("ace/mode/{mode}");
                editor.setReadOnly(true);
                editor.setShowPrintMargin(false);
                editor.renderer.setShowGutter(true);
            }})
        "#,
    );

    maud::html! {
        script { (maud::PreEscaped(js)) }
    }
}

fn infer_ace_mode(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "go" => "golang",
        "c" | "h" => "c_cpp",
        "cpp" | "cc" | "cxx" | "hpp" => "c_cpp",
        "java" => "java",
        "rb" => "ruby",
        "php" => "php",
        "html" | "htm" => "html",
        "css" => "css",
        "json" => "json",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "sh" | "bash" => "sh",
        "sql" => "sql",
        _ => "text",
    }
}
