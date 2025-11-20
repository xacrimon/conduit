mod login;
mod register;

use crate::AppState;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(login::routes())
        .merge(register::routes())
}
