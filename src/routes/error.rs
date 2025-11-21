use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct AppError {
    error: anyhow::Error,
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self {
            error: error.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.error),
        )
            .into_response()
    }
}
