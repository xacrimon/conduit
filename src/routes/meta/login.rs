use axum::Router;
use axum::extract::{Form, Query};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use crate::auth::Session;
use crate::routes::{AppError, shell};
use crate::{AppState, auth, model};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(page_login))
        .route("/login", post(do_login))
}

async fn page_login(session: Option<Session>) -> maud::Markup {
    let markup = maud::html! {
        form method="post" {
            label for="username" { "Username" }
            input .border-solid .border-1 type="text" name="username" required;

            label for="password" { "Password" }
            input .border-solid .border-1 type="password" name="password" required;

            input .text-neutral-50 .bg-blue-500 .border-neutral-700 .border-solid .border-1 type="submit" value="Log in";
        }
    };

    shell::document(markup, "log in", session)
}

#[derive(Deserialize)]
struct LoginQuery {
    redirect: Option<String>,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn do_login(
    mut jar: CookieJar,
    Query(query): Query<LoginQuery>,
    Form(login): Form<LoginForm>,
) -> Result<(CookieJar, Redirect), AppError> {
    let redirect = query.redirect;
    let LoginForm { username, password } = login;

    let user_id = model::user::login(&username, &password).await?;
    let session = model::session::create(user_id).await?;

    let cookie = Cookie::build((auth::COOKIE_NAME, session.token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .expires(session.expires);

    jar = jar.add(cookie);
    let destination = redirect.unwrap_or_else(|| "/".to_string());
    Ok((jar, Redirect::to(&destination)))
}
