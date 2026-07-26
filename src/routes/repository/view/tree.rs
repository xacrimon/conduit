use axum::Router;
use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::git;
use crate::middleware::auth::Session;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/~{user}/{repo}/tree/{*path}", get(page_tree))
}

async fn page_tree(
    state: AppState,
    session: Option<Session>,
    AxumPath((username, repo_name, path)): AxumPath<(String, String, String)>,
) -> Result<Response, AppError> {
    let (repo, user_id) = match super::super::resolve_repo(&state, &username, &repo_name).await? {
        Some(r) => r,
        None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
    };

    if !super::super::can_view(&repo, &session) {
        return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response());
    }

    let is_owner = session.as_ref().is_some_and(|s| s.id == user_id);
    let disk_path = git::repo_disk_path(&state.config, &username, &repo_name);
    let entries = git::ls_tree(&disk_path, &path)
        .await
        .map_err(anyhow::Error::from)?;

    let markup = maud::html! {
        (super::super::repo_header(&username, &repo_name, &repo, is_owner))
        (super::breadcrumbs(&username, &repo_name, &path))
        (tree_content(&username, &repo_name, &path, &entries))
    };

    let title = format!("~{}/{}/{}", username, repo_name, path);
    Ok(shell::document(markup, &title, session).into_response())
}

fn tree_content(
    username: &str,
    repo_name: &str,
    current_path: &str,
    entries: &[git::TreeEntry],
) -> maud::Markup {
    maud::html! {
        div .mt-2 .border .border-gray-200 {
            @for entry in entries {
                @let entry_path = if current_path.is_empty() {
                    entry.name.clone()
                } else {
                    format!("{}/{}", current_path, entry.name)
                };
                div .flex .items-center .px-3 .py-2 .border-b .border-gray-100 .last:border-b-0 .hover:bg-gray-50 {
                    @if entry.kind == "tree" {
                        span .text-blue-500 .mr-2 .text-sm { "/" }
                        a .text-blue-600 .hover:underline href=(format!("/~{}/{}/tree/{}", username, repo_name, entry_path)) {
                            (entry.name)
                        }
                    } @else {
                        span .text-gray-400 .mr-2 .text-sm { " " }
                        a .hover:underline href=(format!("/~{}/{}/blob/{}", username, repo_name, entry_path)) {
                            (entry.name)
                        }
                    }
                }
            }
            @if entries.is_empty() {
                div .px-3 .py-2 .text-gray-500 { "Empty directory." }
            }
        }
    }
}
