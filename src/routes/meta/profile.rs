use axum::Router;
use axum::extract::Form;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::middleware::auth::Session;
use crate::model;
use crate::routes::{AppError, shell};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/meta/profile", get(page_profile))
        .route("/meta/profile", post(do_update_profile))
}

async fn page_profile(state: AppState, session: Session) -> Result<Response, AppError> {
    let profile = model::user::get_profile(&state.db, session.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    let markup = maud::html! {
        (super::meta_nav())

        h2 .text-xl .mt-4 .mb-4 { "Edit your profile" }

        form method="post" {
            div .mb-3 {
                label for="username" .block .mb-1 { "Username" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .bg-gray-100
                    .w-full
                    .max-w-md
                    .p-2
                    .text-gray-600
                    type="text"
                    name="username"
                    value=(profile.username)
                    disabled;
                p .text-sm .text-gray-500 .mt-1 {
                    "Username cannot be changed."
                }
            }

            div .mb-3 {
                label for="display_name" .block .mb-1 { "Display Name" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .max-w-md
                    .p-2
                    type="text"
                    name="display_name"
                    value=(profile.display_name)
                    required;
            }

            div .mb-3 {
                label for="email" .block .mb-1 { "Email" }
                input
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .max-w-md
                    .p-2
                    type="email"
                    name="email"
                    value=(profile.email)
                    required;
            }

            div .mb-3 {
                label for="biography" .block .mb-1 { "Biography" }
                textarea
                    .border-solid
                    .border-1
                    .border-gray-300
                    .w-full
                    .max-w-md
                    .p-2
                    name="biography"
                    rows="4"
                    placeholder="Tell us about yourself..."
                {
                    (profile.biography)
                }
            }

            div .mt-6 {
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
        }
    };

    Ok(shell::document(markup, "profile", session).into_response())
}

#[derive(Deserialize)]
struct UpdateProfileForm {
    display_name: String,
    email: String,
    biography: String,
}

async fn do_update_profile(
    state: AppState,
    session: Session,
    Form(form): Form<UpdateProfileForm>,
) -> Result<Redirect, AppError> {
    model::user::update_profile(
        &state.db,
        session.id,
        form.email.trim(),
        form.display_name.trim(),
        form.biography.trim(),
    )
    .await?;

    Ok(Redirect::to("/meta/profile"))
}
