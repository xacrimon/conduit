mod new;
mod settings;
mod view;

use axum::Router;
use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::middleware::auth::Session;
use crate::routes::{AppError, shell};
use crate::state::AppState;
use crate::{git, model};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/~{user}/{repo}", get(page_repo_root))
        .merge(view::routes())
        .merge(settings::routes())
        .merge(new::routes())
}

async fn page_repo_root(
    state: AppState,
    session: Option<Session>,
    AxumPath((username, repo_name)): AxumPath<(String, String)>,
) -> Result<Response, AppError> {
    let (repo, user_id) = match resolve_repo(&state, &username, &repo_name).await? {
        Some(r) => r,
        None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
    };

    if !can_view(&repo, &session) {
        return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response());
    }

    let is_owner = session.as_ref().is_some_and(|s| s.id == user_id);
    let disk_path = git::repo_disk_path(&state.config, &username, &repo_name);
    let empty = git::is_empty(&disk_path).await;

    let content = if empty {
        empty_repo_content(&state, &username, &repo_name)
    } else {
        let entries = git::ls_tree(&disk_path, "")
            .await
            .map_err(anyhow::Error::from)?;
        tree_content(&username, &repo_name, &entries)
    };

    let markup = maud::html! {
        (repo_header(&username, &repo_name, &repo, is_owner))
        (clone_urls(&state, &username, &repo_name))
        (content)
    };

    let title = format!("~{}/{}", username, repo_name);
    Ok(shell::document(markup, &title, session).into_response())
}

// --- shared helpers ---

pub(crate) async fn resolve_repo(
    state: &AppState,
    username: &str,
    repo_name: &str,
) -> Result<Option<(model::repository::Repository, model::user::UserId)>, AppError> {
    let user_id = match model::user::get_id_by_username(&state.db, username).await? {
        Some(id) => id,
        None => return Ok(None),
    };

    let repo = match model::repository::get_by_owner_and_name(&state.db, user_id, repo_name).await?
    {
        Some(r) => r,
        None => return Ok(None),
    };

    Ok(Some((repo, user_id)))
}

pub(crate) fn can_view(repo: &model::repository::Repository, session: &Option<Session>) -> bool {
    if repo.visibility == "public" {
        return true;
    }
    session.as_ref().is_some_and(|s| s.id == repo.user_id)
}

pub(crate) fn repo_header(
    username: &str,
    repo_name: &str,
    repo: &model::repository::Repository,
    is_owner: bool,
) -> maud::Markup {
    maud::html! {
        div .mb-4 {
            h1 .text-xl {
                a .hover:underline href=(format!("/~{}", username)) { "~" (username) }
                " / "
                a .font-semibold .hover:underline href=(format!("/~{}/{}", username, repo_name)) { (repo_name) }

                span .ml-3 {
                    @if repo.visibility == "public" {
                        span .text-xs .bg-green-100 .text-green-800 .px-2 .py-1 .rounded { "public" }
                    } @else {
                        span .text-xs .bg-gray-100 .text-gray-800 .px-2 .py-1 .rounded { "private" }
                    }
                }
            }
            @if !repo.description.is_empty() {
                p .text-gray-600 .text-sm .mt-1 { (repo.description) }
            }
            @if is_owner {
                div .mt-2 {
                    a .text-sm .text-blue-600 .hover:underline href=(format!("/~{}/{}/settings", username, repo_name)) {
                        "Settings"
                    }
                }
            }
        }
    }
}

fn clone_urls(state: &AppState, username: &str, repo_name: &str) -> maud::Markup {
    //let http_url = format!(
    //    "{}/~{}/{}.git",
    //    state.config.http.public_url, username, repo_name
    //);

    let ssh_host = url::Url::parse(&state.config.http.public_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_owned()))
        .unwrap_or_else(|| "localhost".to_owned());
    let ssh_port = state.config.ssh.port;
    let ssh_url = format!(
        "ssh://git@{}:{}/~{}/{}.git",
        ssh_host, ssh_port, username, repo_name
    );

    maud::html! {
        div .mb-4 .p-3 .bg-gray-50 .border .border-gray-200 {
            p .text-sm .font-semibold .mb-2 { "Clone" }
            div .mb-1 {
                span .text-xs .text-gray-500 { "SSH" }
                code .block .text-sm .bg-white .border .border-gray-200 .p-2 .select-all { (ssh_url) }
            }
            //div {
            //    span .text-xs .text-gray-500 { "HTTP" }
            //    code .block .text-sm .bg-white .border .border-gray-200 .p-2 .select-all { (http_url) }
            //}
        }
    }
}

fn empty_repo_content(state: &AppState, username: &str, repo_name: &str) -> maud::Markup {
    let ssh_host = url::Url::parse(&state.config.http.public_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_owned()))
        .unwrap_or_else(|| "localhost".to_owned());
    let ssh_port = state.config.ssh.port;
    let ssh_url = format!(
        "ssh://git{}:{}/~{}/{}.git",
        ssh_host, ssh_port, username, repo_name
    );

    maud::html! {
        div .mt-4 .p-4 .bg-gray-50 .border .border-gray-200 {
            p .text-gray-600 .mb-4 { "This repository is empty. Push some code to get started:" }
            pre .text-sm .bg-white .border .border-gray-200 .p-3 .overflow-x-auto {
                "git remote add origin " (ssh_url) "\n"
                "git push -u origin main"
            }
        }
    }
}

fn tree_content(username: &str, repo_name: &str, entries: &[git::TreeEntry]) -> maud::Markup {
    maud::html! {
        div .mt-2 .border .border-gray-200 {
            @for entry in entries {
                div .flex .items-center .px-3 .py-2 .border-b .border-gray-100 .last:border-b-0 .hover:bg-gray-50 {
                    @if entry.kind == "tree" {
                        span .text-blue-500 .mr-2 .text-sm { "/" }
                        a .text-blue-600 .hover:underline href=(format!("/~{}/{}/tree/{}", username, repo_name, entry.name)) {
                            (entry.name)
                        }
                    } @else {
                        span .text-gray-400 .mr-2 .text-sm { " " }
                        a .hover:underline href=(format!("/~{}/{}/blob/{}", username, repo_name, entry.name)) {
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
