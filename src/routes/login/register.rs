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
        .route("/register", get(page_register))
        .route("/register", post(do_register))
}

async fn page_register(session: Option<Session>) -> Response {
    if session.is_some() {
        return Redirect::to("/").into_response();
    }

    let markup = maud::html! {
        div .max-w-md {
            h2 .text-xl .mb-4 { "Register" }

            form method="post" {
                div .mb-3 {
                    label for="username" .block .mb-1 { "Username" }
                    input
                        .border-solid
                        .border-1
                        .border-gray-300
                        .w-full
                        .p-2
                        type="text"
                        name="username"
                        required;
                }

                div .mb-3 {
                    label for="email" .block .mb-1 { "Email" }
                    input
                        .border-solid
                        .border-1
                        .border-gray-300
                        .w-full
                        .p-2
                        type="email"
                        name="email"
                        required;
                }

                div .mb-3 {
                    label for="password" .block .mb-1 { "Password" }
                    input
                        .border-solid
                        .border-1
                        .border-gray-300
                        .w-full
                        .p-2
                        type="password"
                        name="password"
                        required;
                }

                div .mt-4 {
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
                        value="Register";
                }
            }

            p .mt-6 .text-gray-600 {
                "Already have an account? "
                a .text-blue-600 .hover:underline href="/login" { "Log in" }
            }
        }
    };

    shell::document(markup, "register", session).into_response()
}

#[derive(Deserialize)]
struct Register {
    username: String,
    email: String,
    password: String,
}

// TODO: form input validation
async fn do_register(
    state: AppState,
    Form(register): Form<Register>,
) -> Result<Redirect, AppError> {
    let Register {
        username,
        email,
        password,
    } = register;
    model::user::create(&state.db, &username, &email, &password).await?;
    Ok(Redirect::to("/login"))
}
