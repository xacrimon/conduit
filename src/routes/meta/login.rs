use crate::AppState;
use crate::model;
use crate::routes::document;
use crate::routes::error::AppError;
use axum::extract::Form;
use axum::response::Redirect;
use axum::routing::{get, post};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/login", get(page_login))
        .route("/login", post(do_login))
}

async fn page_login() -> maud::Markup {
    let markup = maud::html! {
        form action="/login" method="post" {
            label for="username" { "Username:" }
            input type="text" name="username" required;

            label for="password" { "Password:" }
            input type="password" name="password" required;

            input type="submit" value="Log in";
        }
    };

    document(markup, "login")
}

#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
    redirect: Option<String>,
}

async fn do_login(
    mut jar: CookieJar,
    Form(login): Form<Login>,
) -> Result<(CookieJar, Redirect), AppError> {
    let Login {
        username,
        password,
        redirect,
    } = login;

    let user_id = model::user::login(&username, &password).await?;
    let session = model::session::create(user_id).await?;

    let cookie = Cookie::build(("conduit_session", session.token))
        .http_only(true)
        .expires(session.expires);

    jar = jar.add(cookie);
    let destination = redirect.unwrap_or_else(|| "/".to_string());
    Ok((jar, Redirect::to(&destination)))
}
