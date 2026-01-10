mod login;
mod logout;
mod register;

use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(login::routes())
        .merge(logout::routes())
        .merge(register::routes())
}
