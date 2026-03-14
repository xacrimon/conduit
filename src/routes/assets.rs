use std::sync::LazyLock;

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

const ASSET_MAP_DATA: &str = include_str!(concat!(env!("OUT_DIR"), "/asset_map.txt"));

static ASSET_MAP: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    ASSET_MAP_DATA
        .lines()
        .map(|line| {
            let mut parts = line.trim().splitn(2, '=');
            let name = parts.next().unwrap().to_string();
            let asset_name = parts.next().unwrap().to_string();
            (name, asset_name)
        })
        .collect()
});

pub fn path(name: &str) -> String {
    let path = ASSET_MAP
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, asset_name)| asset_name.as_str())
        .unwrap();

    format!("/assets/{}", path)
}

pub fn routes() -> Router<AppState> {
    let css_route = format!("/assets/{}", CSS_ASSET_NAME);

    Router::new()
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .route(&css_route, get(handle_css))
        .nest_service("/assets", ServeDir::new("public/assets"))
        .layer(axum::middleware::from_fn(header_middleware))
        .layer(axum::middleware::from_fn(map_middleware))
}

async fn handle_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css")], CSS)
}

async fn map_middleware(mut request: Request, next: Next) -> Response {
    let path = request.uri().path();

    if path.starts_with("/assets/") && !path.starts_with("/assets/lib/") {
        let asset = path.strip_prefix("/assets/").unwrap();

        if let Some((mapped, _)) = ASSET_MAP.iter().find(|(_, asset_name)| asset_name == asset) {
            let real_path = format!("/assets/{}", mapped);
            *request.uri_mut() = real_path.parse().unwrap();
        }
    }

    next.run(request).await
}

async fn header_middleware(request: Request, next: Next) -> Response {
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
        _ => "public, max-age=2592000, immutable",
    };

    Some(HeaderValue::from_static(directives))
}
