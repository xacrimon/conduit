mod profile;

use crate::AppState;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().merge(profile::routes())
}
