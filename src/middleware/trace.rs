use std::convert::Infallible;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

fn request_id() -> u64 {
    cfg_select! {
        debug_assertions => {
            use std::sync::atomic::{Ordering, AtomicU64};

            static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
            REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        }
        _ => { rand::random() }
    }
}

pub async fn middleware(
    request: Request,
    next: Next,
) -> Result<Response, Infallible> {
    let request_id = request_id();
    let span = tracing::info_span!("web request", web_request_id = request_id);
    
    let method = request.method();
    let uri = request.uri();
    tracing::info!(parent: &span, "received request: {} {}", method, uri);

    let response = next.run(request).instrument(span).await;
    Ok(response)
}
