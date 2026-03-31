mod settings;
mod view;

use std::path::Path;

use axum::Router;
use axum::extract::Form;
use axum::response::Redirect;
use axum::routing::{get, post};
use serde::Deserialize;
use tokio::process::Command;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;
use crate::utils::re;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(view::routes())
        .merge(settings::routes())
        .route("/repository/new", get(page_new))
        .route("/repository/new", post(do_create))
}

async fn page_new(session: Session) -> maud::Markup {
    let markup = maud::html! {
        h2 .text-xl .mb-4 { "New Repository" }

        form method="post" {
            div .mb-3 {
                label for="name" .block .mb-1 { "Name" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .p-2
                    type="text"
                    name="name"
                    placeholder="my-project"
                    pattern="[a-zA-Z0-9][a-zA-Z0-9._-]*"
                    required;
                p .text-xs .text-gray-500 .mt-1 {
                    "Letters, numbers, hyphens, dots, and underscores. Must start with a letter or number."
                }
            }

            div .mb-3 {
                label for="description" .block .mb-1 { "Description" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .p-2
                    type="text"
                    name="description"
                    placeholder="A short description";
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
                    option value="private" { "Private - only you" }
                }
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
                value="Create Repository";
        }
    };

    shell::document(markup, "new repository", session)
}

#[derive(Deserialize)]
struct CreateForm {
    name: String,
    description: String,
    visibility: String,
}

fn valid_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && re!(r"^[a-zA-Z0-9][a-zA-Z0-9._-]*$").is_match(name)
        && name != "settings"
}

async fn do_create(
    state: AppState,
    session: Session,
    Form(form): Form<CreateForm>,
) -> Result<Redirect, AppError> {
    let name = form.name.trim().to_owned();
    if !valid_repo_name(&name) {
        return Err(anyhow::anyhow!("Invalid repository name").into());
    }

    let visibility = match form.visibility.as_str() {
        "private" => "private",
        _ => "public",
    };

    let description = form.description.trim().to_owned();

    model::repository::create(&state.db, session.id, &name, &description, visibility).await?;

    let disk_path = repo_disk_path(&state.config, &session.username, &name);
    init_bare_repo(&disk_path).await?;

    Ok(Redirect::to(&format!("/~{}/{}", session.username, name)))
}

pub fn repo_disk_path(
    config: &crate::config::Config,
    username: &str,
    name: &str,
) -> std::path::PathBuf {
    config
        .git
        .repository_path
        .join(username)
        .join(format!("{}.git", name))
}

async fn init_bare_repo(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let output = Command::new("git")
        .args(["init", "--bare"])
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!(
            "git init --bare failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
