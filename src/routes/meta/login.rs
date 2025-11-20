use crate::AppState;
use crate::auth;
use crate::model;
use crate::routes::AppError;
use crate::routes::document;
use axum::extract::{Form, Query};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/login", get(page_login))
        .route("/login", post(do_login))
}

#[derive(Deserialize)]
struct Login {
    redirect: Option<String>,
}

async fn page_login(Query(login): Query<Login>) -> maud::Markup {
    let redirect_input = match login.redirect {
        Some(redirect) => {
            maud::html! {
                input type="hidden" name="redirect" value=(redirect);
            }
        }
        None => maud::html! {},
    };

    let markup = maud::html! {
        form action="/login" method="post" {
            label for="username" { "Username:" }
            input type="text" name="username" required;

            label for="password" { "Password:" }
            input type="password" name="password" required;

            (redirect_input)
            input type="submit" value="Log in";
        }
    };

    document(markup, "login")
}

#[derive(Deserialize)]
struct DoLogin {
    username: String,
    password: String,
    redirect: Option<String>,
}

async fn do_login(
    mut jar: CookieJar,
    Form(login): Form<DoLogin>,
) -> Result<(CookieJar, Redirect), AppError> {
    let DoLogin {
        username,
        password,
        redirect,
    } = login;

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
