use axum::Router;
use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::git;
use crate::middleware::auth::Session;
use crate::routes::{AppError, ace, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/~{user}/{repo}/blob/{*path}", get(page_blob))
}

async fn page_blob(
    state: AppState,
    session: Option<Session>,
    AxumPath((username, repo_name, path)): AxumPath<(String, String, String)>,
) -> Result<Response, AppError> {
    let (repo, _user_id) = match super::super::resolve_repo(&state, &username, &repo_name).await? {
        Some(r) => r,
        None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
    };

    if !super::super::can_view(&repo, &session) {
        return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response());
    }

    let disk_path = git::repo_disk_path(&state.config, &username, &repo_name);
    let blob = git::show_blob(&disk_path, &path)
        .await
        .map_err(anyhow::Error::from)?;

    let content = match blob {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) => {
                let filename = path.rsplit('/').next().unwrap_or(&path);
                let mode = ace::infer_mode(filename);
                maud::html! {
                    div #editor .relative .w-full style="height: 600px;" .border-solid .border-1 .border-gray-300 {
                        (text)
                    }
                    (ace::readonly("editor", mode))
                }
            }
            Err(_) => maud::html! {
                p .text-gray-500 { "Binary file cannot be displayed." }
            },
        },
        None => maud::html! {
            p .text-gray-500 { "File not found." }
        },
    };

    let markup = maud::html! {
        (super::breadcrumbs(&username, &repo_name, &path))
        (content)
    };

    let title = format!("~{}/{} - {}", username, repo_name, path);
    Ok(shell::document_with(markup, &title, session, ace::script()).into_response())
}
