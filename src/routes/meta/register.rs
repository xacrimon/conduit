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
        form method="post" {
            label for="username" { "Username" }
            input .border-solid .border-1 type="text" name="username" required;

            label for="username" { "Email" }
            input .border-solid .border-1 type="text" name="email" required;

            label for="password" { "Password" }
            input .border-solid .border-1 type="password" name="password" required;

            input .text-neutral-50 .bg-blue-500 .border-neutral-700 .border-solid .border-1 type="submit" value="Register";
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
