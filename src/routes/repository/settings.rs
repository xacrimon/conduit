use axum::Router;
use axum::extract::{Form, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/~{user}/{repo}/settings", get(page_settings))
        .route("/~{user}/{repo}/settings", post(do_update))
        .route("/~{user}/{repo}/settings/delete", post(do_delete))
}

async fn page_settings(
    state: AppState,
    session: Session,
    Path((username, repo_name)): Path<(String, String)>,
) -> Result<Response, AppError> {
    if session.username != username {
        return Ok((StatusCode::NOT_FOUND, "Not found").into_response());
    }

    let repo =
        match model::repository::get_by_owner_and_name(&state.db, session.id, &repo_name).await? {
            Some(r) => r,
            None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
        };

    let markup = maud::html! {
        (super::repo_header(&username, &repo_name, &repo, true))

        form method="post" .mb-8 {
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
                    value=(repo.description);
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
                    option value="public" selected[repo.visibility == "public"] {
                        "Public - visible to everyone"
                    }
                    option value="private" selected[repo.visibility == "private"] {
                        "Private - only you"
                    }
                }
            }

            div .mb-3 {
                label for="default_branch" .block .mb-1 { "Default Branch" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .p-2
                    type="text"
                    name="default_branch"
                    value=(repo.default_branch);
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
                value="Save Changes";
        }

        div .border .border-red-300 .p-4 {
            h3 .text-lg .text-red-700 .mb-2 { "Danger Zone" }
            p .text-sm .text-gray-600 .mb-3 {
                "Deleting a repository is permanent and cannot be undone."
            }
            form method="post" action=(format!("/~{}/{}/settings/delete", username, repo_name)) {
                button
                    .text-neutral-50
                    .bg-red-600
                    .hover:bg-red-700
                    .border-red-800
                    .border-solid
                    .border-1
                    .px-4
                    .py-2
                    .cursor-pointer
                    type="submit"
                    onclick="return confirm('Are you sure you want to delete this repository?')"
                {
                    "Delete Repository"
                }
            }
        }
    };

    let title = format!("~{}/{} - settings", username, repo_name);
    Ok(shell::document(markup, &title, session).into_response())
}

#[derive(Deserialize)]
struct UpdateForm {
    description: String,
    visibility: String,
    default_branch: String,
}

async fn do_update(
    state: AppState,
    session: Session,
    Path((username, repo_name)): Path<(String, String)>,
    Form(form): Form<UpdateForm>,
) -> Result<Response, AppError> {
    if session.username != username {
        return Ok((StatusCode::FORBIDDEN, "Forbidden").into_response());
    }

    let repo =
        match model::repository::get_by_owner_and_name(&state.db, session.id, &repo_name).await? {
            Some(r) => r,
            None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
        };

    let visibility = match form.visibility.as_str() {
        "private" => "private",
        _ => "public",
    };

    let default_branch = form.default_branch.trim();
    let default_branch = if default_branch.is_empty() {
        "main"
    } else {
        default_branch
    };

    model::repository::update(
        &state.db,
        repo.id,
        session.id,
        form.description.trim(),
        visibility,
        default_branch,
    )
    .await?;

    Ok(Redirect::to(&format!("/~{}/{}/settings", username, repo_name)).into_response())
}

async fn do_delete(
    state: AppState,
    session: Session,
    Path((username, repo_name)): Path<(String, String)>,
) -> Result<Response, AppError> {
    if session.username != username {
        return Ok((StatusCode::FORBIDDEN, "Forbidden").into_response());
    }

    let repo =
        match model::repository::get_by_owner_and_name(&state.db, session.id, &repo_name).await? {
            Some(r) => r,
            None => return Ok((StatusCode::NOT_FOUND, "Repository not found").into_response()),
        };

    model::repository::delete(&state.db, repo.id, session.id).await?;

    // Remove the bare repo from disk
    let disk_path = crate::git::repo_disk_path(&state.config, &username, &repo_name);
    let _ = tokio::fs::remove_dir_all(&disk_path).await;

    Ok(Redirect::to(&format!("/~{}", username)).into_response())
}
