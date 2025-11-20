mod login;
mod logout;
mod register;

use crate::AppState;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(login::routes())
        .merge(logout::routes())
        .merge(register::routes())
}
