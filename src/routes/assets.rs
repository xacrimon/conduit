use std::path::{Path as FsPath, PathBuf};
use std::sync::LazyLock;

use axum::body::Body;
use axum::extract::Path;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use axum::response::Response;
use axum::routing::get;
use axum::{Router, http};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
use http::header::{self};
use mime_guess::{Mime, mime};
use sha2::{Digest, Sha256};
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;

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
        .route("/favicon.ico", get(handle_favicon))
        .route(&css_route, get(handle_css))
        .route("/assets/{*key}", get(handle_asset))
}

async fn handle_css(headers: HeaderMap) -> Response {
    prepare_response("", CSS.to_owned().into_bytes(), &headers, mime::TEXT_CSS)
}

async fn handle_favicon(headers: HeaderMap) -> Response {
    serve_asset(&PathBuf::from("public/favicon.ico"), &headers).await
}

async fn handle_asset(Path(path): Path<String>, headers: HeaderMap) -> Response {
    let mut asset = &path;

    if !path.starts_with("lib/") {
        if let Some((mapped, _)) = ASSET_MAP.iter().find(|(_, asset_name)| asset_name == &path) {
            asset = mapped;
        } else {
            return Response::builder()
                .status(404)
                .body("Not Found".into())
                .unwrap();
        }
    }

    let real_path = PathBuf::from(format!("public/assets/{}", asset));
    serve_asset(&real_path, &headers).await
}

async fn serve_asset(path: &FsPath, request_headers: &HeaderMap) -> Response {
    let mut options = OpenOptions::new();
    options.read(true);

    let mut file = match options.open(&path).await {
        Ok(file) => file,
        Err(_) => {
            return Response::builder()
                .status(404)
                .body("Not Found".into())
                .unwrap();
        }
    };

    let meta = file.metadata().await.unwrap();
    if !meta.is_file() {
        todo!()
    }

    let len = meta.len() as usize;
    let mut buf = Vec::with_capacity(len);
    let read = file.read_to_end(&mut buf).await.unwrap();
    assert!(read == len);

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let content_type = mime_guess::from_ext(extension).first_or_octet_stream();

    prepare_response("", buf, request_headers, content_type)
}

fn prepare_response(
    path: &str,
    data: Vec<u8>,
    request_headers: &HeaderMap,
    content_type: Mime,
) -> Response {
    let len = data.len();
    let etag = compute_etag(&data);
    let cache_directive = match path {
        "/favicon.ico" => "public, max-age=86400",
        _ => "public, max-age=2592000, immutable",
    };

    let if_none_match = request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());

    let mut status = 200;
    let mut body = data.into();

    if Some(etag.as_str()) == if_none_match {
        status = 304;
        body = Body::empty();
    }

    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type.to_string())
        .header(header::CONTENT_LENGTH, len)
        .header(header::ETAG, etag)
        .header(header::CACHE_CONTROL, cache_directive)
        .body(body)
        .unwrap()
}

fn compute_etag(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    BASE64_URL_SAFE_NO_PAD.encode(hash)
}
