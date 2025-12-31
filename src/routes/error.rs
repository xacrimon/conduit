use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

// TODO: improve error handling
#[derive(Debug, Error)]
pub enum AppError {
    #[error("internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Internal(err) => {
                let message = format!("{}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
            }
        }
    }
}
