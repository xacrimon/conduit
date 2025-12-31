mod profile;

use axum::Router;

use crate::state::AppStateRef;

pub fn routes() -> Router<AppStateRef> {
    Router::new().merge(profile::routes())
}
