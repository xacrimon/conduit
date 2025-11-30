use std::any::Any;

use axum::response::{IntoResponse, Response};
use tower_http::catch_panic::CatchPanicLayer;

fn handle_panic(_error: Box<dyn Any + Send + 'static>) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Internal Server Error",
    )
        .into_response()
}

pub fn middleware() -> CatchPanicLayer<fn(Box<dyn Any + Send + 'static>) -> Response> {
    CatchPanicLayer::custom(handle_panic)
}
