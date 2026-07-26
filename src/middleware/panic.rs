use std::panic::AssertUnwindSafe;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures_util::FutureExt;
use tower::{Layer, Service};

#[derive(Clone, Copy)]
pub struct CatchPanicLayer;

impl<S> Layer<S> for CatchPanicLayer {
    type Service = CatchPanicService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CatchPanicService { inner }
    }
}

#[derive(Clone)]
pub struct CatchPanicService<S> {
    inner: S,
}

impl<S, B> Service<axum::extract::Request<B>> for CatchPanicService<S>
where
    S: Service<axum::extract::Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    B: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::extract::Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            match AssertUnwindSafe(inner.call(req)).catch_unwind().await {
                Ok(result) => result,
                Err(_) => Ok(
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
                ),
            }
        })
    }
}

pub fn middleware() -> CatchPanicLayer {
    CatchPanicLayer
}
