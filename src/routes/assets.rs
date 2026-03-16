use std::path::{Path as FsPath, PathBuf};
use std::sync::LazyLock;

use axum::body::Body;
use axum::extract::Path;
use axum::http::Method;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, http};
use http::header::{self};
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
        .route("/favicon.ico", get(handle_favicon).head(handle_favicon))
        .route(&css_route, get(handle_css))
        .route("/assets/{*key}", get(handle_asset).head(handle_asset))
}

async fn handle_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css")], CSS)
}

async fn handle_favicon(method: Method) -> impl IntoResponse {
    serve_asset(&PathBuf::from("public/favicon.ico"), method == Method::HEAD).await
}

async fn handle_asset(Path(path): Path<String>, method: Method) -> Response {
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
    serve_asset(&real_path, method == Method::HEAD).await
}

async fn serve_asset(path: &FsPath, is_head: bool) -> Response {
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

    let body = if !is_head {
        let mut buf = Vec::with_capacity(len);
        let read = file.read_to_end(&mut buf).await.unwrap();
        assert!(read == len);
        buf.into()
    } else {
        Body::empty()
    };

    let cache_directive = match path.to_str() {
        Some("public/favicon.ico") => "public, max-age=86400",
        _ => "public, max-age=2592000, immutable",
    };

    Response::builder()
        .header(
            CONTENT_TYPE,
            mime_guess::from_path(&path)
                .first_or_octet_stream()
                .as_ref(),
        )
        .header(header::CONTENT_LENGTH, len)
        .header(header::CACHE_CONTROL, cache_directive)
        .body(body)
        .unwrap()
}
