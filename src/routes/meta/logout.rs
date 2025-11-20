use crate::AppState;
use crate::auth;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::CookieJar;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/logout", get(page_logout))
}

async fn page_logout(mut jar: CookieJar) -> (CookieJar, Redirect) {
    jar = jar.remove(auth::COOKIE_NAME);
    (jar, Redirect::to("/"))
}
