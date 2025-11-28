use axum::Router;
use axum::extract::{Form, State};
use axum::response::Redirect;
use axum::routing::{get, post};
use serde::Deserialize;

use crate::auth::Session;
use crate::routes::{AppError, shell};
use crate::{AppState, model};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", get(page_register))
        .route("/register", post(do_register))
}

async fn page_register(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        form method="post" {
            label for="username" { "Username" }
            input .border-solid .border-1 type="text" name="username" required;

            label for="password" { "Password" }
            input .border-solid .border-1 type="password" name="password" required;

            input .text-neutral-50 .bg-blue-500 .border-neutral-700 .border-solid .border-1 type="submit" value="Register";
        }
    };

    shell::document(markup, "register", session)
}

#[derive(Deserialize)]
struct Register {
    username: String,
    password: String,
}

async fn do_register(
    State(state): State<AppState>,
    Form(register): Form<Register>,
) -> Result<Redirect, AppError> {
    let Register { username, password } = register;
    model::user::create(&state.db, &username, &password).await?;
    Ok(Redirect::to("/login"))
}
