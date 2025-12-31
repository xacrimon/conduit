use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::CookieJar;

use crate::middleware::auth;
use crate::state::AppStateRef;

pub fn routes() -> Router<AppStateRef> {
    Router::new().route("/logout", get(page_logout))
}

async fn page_logout(mut jar: CookieJar) -> (CookieJar, Redirect) {
    jar = jar.remove(auth::COOKIE_NAME);
    (jar, Redirect::to("/"))
}
