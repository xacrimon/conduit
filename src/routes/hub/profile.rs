use axum::Router;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/~{name}", get(page_profile))
}

#[derive(Deserialize)]
struct ProfileQuery {
    tab: Option<ProfileTab>,
}

#[derive(Deserialize)]
enum ProfileTab {
    #[serde(rename = "pastes")]
    Pastes,
}

async fn page_profile(
    state: AppState,
    session: Option<Session>,
    Path(name): Path<String>,
    Query(query): Query<ProfileQuery>,
) -> Result<Response, AppError> {
    let user_id = match model::user::get_id_by_username(&state.db, &name).await? {
        Some(id) => id,
        None => return Ok((StatusCode::NOT_FOUND, "User not found").into_response()),
    };

    let profile = model::user::get_profile(&state.db, user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User profile missing"))?;

    let (tab_name, content) = match query.tab {
        Some(ProfileTab::Pastes) => (
            "pastes",
            tab_pastes(&state, &session, &name, user_id).await?,
        ),
        None => ("overview", tab_overview(&profile)),
    };

    let markup = maud::html! {
        (profile_header(&profile))
        (hub_nav(&name, tab_name))
        (content)
    };

    let title = match tab_name {
        "overview" => format!("~{}", name),
        other => format!("~{} - {}", name, other),
    };

    Ok(shell::document(markup, &title, session).into_response())
}

fn tab_overview(profile: &model::user::UserProfile) -> maud::Markup {
    maud::html! {
        @if !profile.biography.is_empty() {
            div .mt-4 {
                p .text-gray-700 { (profile.biography) }
            }
        } @else {
            p .text-gray-500 .mt-4 { "This user hasn't written a biography yet." }
        }
    }
}

async fn tab_pastes(
    state: &AppState,
    session: &Option<Session>,
    name: &str,
    user_id: model::user::UserId,
) -> Result<maud::Markup, AppError> {
    let is_owner = session.as_ref().is_some_and(|s| s.id == user_id);
    let all_pastes = model::paste::get_user_pastes(&state.db, user_id).await?;
    let pastes: Vec<_> = if is_owner {
        all_pastes
    } else {
        all_pastes
            .into_iter()
            .filter(|p| p.visibility == "public")
            .collect()
    };

    Ok(maud::html! {
        div .mt-4 {
            @if pastes.is_empty() {
                p .text-gray-500 { "No public pastes." }
            } @else {
                @for paste in &pastes {
                    div .border-solid .border-1 .border-gray-300 .p-3 .mb-2 {
                        div .flex .items-center {
                            a .font-mono .text-blue-600 .hover:underline href=(format!("/~{}/paste/{}", name, paste.id)) {
                                (paste.filename)
                            }
                            span .text-gray-500 .text-sm .ml-3 {
                                (paste.id)
                            }
                            @if is_owner {
                                span .ml-3 {
                                    @if paste.visibility == "public" {
                                        span .text-xs .bg-green-100 .text-green-800 .px-2 .py-1 .rounded { "public" }
                                    } @else if paste.visibility == "unlisted" {
                                        span .text-xs .bg-yellow-100 .text-yellow-800 .px-2 .py-1 .rounded { "unlisted" }
                                    } @else {
                                        span .text-xs .bg-gray-100 .text-gray-800 .px-2 .py-1 .rounded { "private" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn profile_header(profile: &model::user::UserProfile) -> maud::Markup {
    maud::html! {
        div .mb-4 {
            h1 .text-xl {
                (profile.display_name)
            }
            span .text-gray-500 .text-sm {
                "~" (profile.username)
            }
        }
    }
}

fn hub_nav(username: &str, current: &str) -> maud::Markup {
    let base = format!("/~{}", username);
    let pastes = format!("/~{}?tab=pastes", username);
    shell::subnav(&[("overview", &base), ("pastes", &pastes)], current)
}
