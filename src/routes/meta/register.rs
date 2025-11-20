use super::document;
use crate::AppState;
use crate::auth::Session;
use crate::model;
use crate::routes::AppError;
use axum::extract::Form;
use axum::response::Redirect;
use axum::routing::{get, post};
use serde::Deserialize;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/register", get(page_register))
        .route("/register", post(do_register))
}

async fn page_register(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        form method="post" {
            label for="username" { "Username:" }
            input type="text" name="username" required;

            label for="password" { "Password:" }
            input type="password" name="password" required;

            input type="submit" value="Register";
        }
    };

    document(markup, "register", session)
}

#[derive(Deserialize)]
struct Register {
    username: String,
    password: String,
}

async fn do_register(Form(register): Form<Register>) -> Result<Redirect, AppError> {
    let Register { username, password } = register;
    model::user::create(&username, &password).await?;
    Ok(Redirect::to("/login"))
}
