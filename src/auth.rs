use axum::extract::{FromRequestParts, OptionalFromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::{Redirect, Response};
use axum_extra::extract::cookie::CookieJar;
use url::form_urlencoded;

use crate::model::user::UserId;
use crate::routes::AppError;
use crate::{AppState, model};

pub const COOKIE_NAME: &str = "conduit_session";

#[derive(Debug, Clone)]
pub struct Session {
    pub id: UserId,
    pub username: String,
}

impl FromRequestParts<AppState> for Session {
    type Rejection = Redirect;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(session) = parts.extensions.get::<Session>().cloned() {
            Ok(session)
        } else {
            let path_and_query = parts.uri.path_and_query().unwrap().as_str();
            let encoded = form_urlencoded::byte_serialize(path_and_query.as_bytes());
            let destination = format!("/login?redirect={}", encoded.collect::<String>());
            Err(Redirect::to(&destination))
        }
    }
}

impl OptionalFromRequestParts<AppState> for Session {
    type Rejection = ();

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<Session>().cloned())
    }
}

pub async fn middleware(
    State(state): State<AppState>,
    mut jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<(CookieJar, Response), AppError> {
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        let token = cookie.value();
        let maybe_session = model::session::get_by_token(&state.db, token).await?;

        if let Some(session) = maybe_session
            && session.expires > time::OffsetDateTime::now_utc()
        {
            let user = model::user::get_by_id(&state.db, session.user_id)
                .await?
                .unwrap();

            let auth_session = Session {
                id: user.id,
                username: user.username,
            };

            request.extensions_mut().insert(auth_session);
        } else {
            let cookie = cookie.clone();
            jar = jar.remove(cookie);
        }
    }

    let response = next.run(request).await;
    Ok((jar, response))
}
