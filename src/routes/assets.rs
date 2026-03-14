use axum::extract::Request;
use axum::http::Method;
use axum::http::header::CONTENT_TYPE;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, http};
use http::header::{self, HeaderValue};
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/index.css"));
pub const CSS_ASSET_NAME: &str = env!("CONDUIT_CSS_ASSET_NAME");

pub fn routes() -> Router<AppState> {
    let css_route = format!("/assets/{}", CSS_ASSET_NAME);

    Router::new()
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .route(&css_route, get(handle_css))
        .nest_service("/assets", ServeDir::new("public/assets"))
        .layer(axum::middleware::from_fn(middleware))
}

async fn handle_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css")], CSS)
}

async fn middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let cache_control = cache_control_for(request.uri().path());
    let mut response = next.run(request).await;

    if method != Method::GET || response.status() != http::StatusCode::OK {
        return response;
    }

    if let Some(cache_control) = cache_control {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, cache_control);
    }

    response
}

fn cache_control_for(path: &str) -> Option<HeaderValue> {
    let asset = path.strip_prefix("/assets/").unwrap_or(path);

    let directives = match asset {
        "/favicon.ico" => "public, max-age=86400",
        CSS_ASSET_NAME => "public, max-age=2592000, immutable",
        "lib/htmx-2.0.8.js" => "public, max-age=2592000, immutable",
        "autoreload.js" => "public, max-age=2592000",
        _ if asset.starts_with("lib/ace-1.43.4/") => "public, max-age=2592000, immutable",
        _ => return None,
    };

    Some(HeaderValue::from_static(directives))
}
