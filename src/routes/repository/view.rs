use std::path::Path;

use axum::Router;
use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::process::Command;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/~{user}/{repo}", get(page_repo_root))
        .route("/~{user}/{repo}/tree/{*path}", get(page_tree))
        .route("/~{user}/{repo}/blob/{*path}", get(page_blob))
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
    let disk_path = super::repo_disk_path(&state.config, &username, &repo_name);
    let empty = git_is_empty(&disk_path).await;

    let content = if empty {
        empty_repo_content(&state, &username, &repo_name)
    } else {
        let entries = git_ls_tree(&disk_path, "").await?;
        tree_content(&username, &repo_name, "", &entries)
    };

    let markup = maud::html! {
        (repo_header(&username, &repo_name, &repo, is_owner))
        (clone_urls(&state, &username, &repo_name))
        (content)
    };

    let title = format!("~{}/{}", username, repo_name);
    Ok(shell::document(markup, &title, session).into_response())
}

async fn page_tree(
    state: AppState,
    session: Option<Session>,
    AxumPath((username, repo_name, path)): AxumPath<(String, String, String)>,
) -> Result<Response, AppError> {
    let (repo, user_id) = match resolve_repo(&state, &username, &repo_name).await? {
        Some(r) => r,
        None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
    };

    if !can_view(&repo, &session) {
        return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response());
    }

    let is_owner = session.as_ref().is_some_and(|s| s.id == user_id);
    let disk_path = super::repo_disk_path(&state.config, &username, &repo_name);
    let entries = git_ls_tree(&disk_path, &path).await?;

    let markup = maud::html! {
        (repo_header(&username, &repo_name, &repo, is_owner))
        (breadcrumbs(&username, &repo_name, &path))
        (tree_content(&username, &repo_name, &path, &entries))
    };

    let title = format!("~{}/{}/{}", username, repo_name, path);
    Ok(shell::document(markup, &title, session).into_response())
}

async fn page_blob(
    state: AppState,
    session: Option<Session>,
    AxumPath((username, repo_name, path)): AxumPath<(String, String, String)>,
) -> Result<Response, AppError> {
    let (repo, _user_id) = match resolve_repo(&state, &username, &repo_name).await? {
        Some(r) => r,
        None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
    };

    if !can_view(&repo, &session) {
        return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response());
    }

    let disk_path = super::repo_disk_path(&state.config, &username, &repo_name);
    let blob = git_show_blob(&disk_path, &path).await?;

    let content = match blob {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) => {
                let filename = path.rsplit('/').next().unwrap_or(&path);
                let mode = infer_ace_mode(filename);
                maud::html! {
                    div #editor .relative .w-full style="height: 600px;" .border-solid .border-1 .border-gray-300 {
                        (text)
                    }
                    (ace_readonly("editor", mode))
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
        (breadcrumbs(&username, &repo_name, &path))
        (content)
    };

    let title = format!("~{}/{} - {}", username, repo_name, path);
    Ok(shell::document_with(markup, &title, session, ace_script()).into_response())
}

// --- helpers ---

async fn resolve_repo(
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

fn can_view(repo: &model::repository::Repository, session: &Option<Session>) -> bool {
    if repo.visibility == "public" {
        return true;
    }
    session.as_ref().is_some_and(|s| s.id == repo.user_id)
}

pub fn repo_header(
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
        "git@{}:{}/~{}/{}.git",
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
        "git{}:{}/~{}/{}.git",
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

struct TreeEntry {
    kind: String,
    name: String,
}

fn tree_content(
    username: &str,
    repo_name: &str,
    current_path: &str,
    entries: &[TreeEntry],
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

fn breadcrumbs(username: &str, repo_name: &str, path: &str) -> maud::Markup {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    maud::html! {
        div .mb-3 .text-sm {
            a .text-blue-600 .hover:underline href=(format!("/~{}/{}", username, repo_name)) { (repo_name) }
            @for (i, part) in parts.iter().enumerate() {
                " / "
                @if i == parts.len() - 1 {
                    span .font-semibold { (part) }
                } @else {
                    @let sub_path = parts[..=i].join("/");
                    a .text-blue-600 .hover:underline href=(format!("/~{}/{}/tree/{}", username, repo_name, sub_path)) {
                        (part)
                    }
                }
            }
        }
    }
}

// --- git operations ---

async fn git_is_empty(repo_path: &Path) -> bool {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .await;

    match output {
        Ok(o) => !o.status.success(),
        Err(_) => true,
    }
}

async fn git_ls_tree(repo_path: &Path, path: &str) -> Result<Vec<TreeEntry>, AppError> {
    let tree_ref = if path.is_empty() {
        "HEAD".to_owned()
    } else {
        format!("HEAD:{}", path)
    };

    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo_path)
        .args(["ls-tree", &tree_ref])
        .output()
        .await
        .map_err(anyhow::Error::from)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout).map_err(anyhow::Error::from)?;
    let mut entries: Vec<TreeEntry> = stdout
        .lines()
        .filter_map(|line| {
            let (meta, name) = line.split_once('\t')?;
            let parts: Vec<&str> = meta.split_whitespace().collect();
            if parts.len() != 3 {
                return None;
            }
            Some(TreeEntry {
                kind: parts[1].to_owned(),
                name: name.to_owned(),
            })
        })
        .collect();

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_tree = a.kind == "tree";
        let b_is_tree = b.kind == "tree";
        b_is_tree.cmp(&a_is_tree).then(a.name.cmp(&b.name))
    });

    Ok(entries)
}

async fn git_show_blob(repo_path: &Path, path: &str) -> Result<Option<Vec<u8>>, AppError> {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo_path)
        .args(["show", &format!("HEAD:{}", path)])
        .output()
        .await
        .map_err(anyhow::Error::from)?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(output.stdout))
}

// --- ace editor ---

fn ace_script() -> maud::Markup {
    maud::html! {
        script defer src="/assets/lib/ace-1.43.4/ace.js" {}
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
