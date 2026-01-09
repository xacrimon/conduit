use std::convert::Infallible;
use std::ops::Deref;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::config::Config;

pub struct AppStateInner {
    pub db: PgPool,
    pub config: Config,
    pub cancel_token: CancellationToken,
    pub task_tracker: TaskTracker,
}

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

impl AppState {
    pub fn new(state: AppStateInner) -> Self {
        Self(Arc::new(state))
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<AppState> for AppState {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(state.clone())
    }
}
