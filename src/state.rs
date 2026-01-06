use std::convert::Infallible;
use std::ops::Deref;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sqlx::PgPool;

use crate::config::Config;

pub struct AppState {
    pub db: PgPool,
    pub config: Config,
}

#[derive(Clone)]
pub struct AppStateRef(Arc<AppState>);

impl AppStateRef {
    pub fn new(state: AppState) -> Self {
        Self(Arc::new(state))
    }
}

impl Deref for AppStateRef {
    type Target = AppState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<AppStateRef> for AppStateRef {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppStateRef,
    ) -> Result<Self, Self::Rejection> {
        Ok(state.clone())
    }
}
