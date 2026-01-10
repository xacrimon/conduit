use std::collections::HashMap;
use std::path::PathBuf;

use axum::Router;
use axum::body::Body;
use axum::extract::{Json, Path as AxumPath};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

use crate::model;
use crate::routes::AppError;
use crate::state::AppState;
use crate::utils::re;

const LFS_CONTENT_TYPE: &str = "application/vnd.git-lfs+json";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/~{user}/{repo}/info/lfs/objects/batch", post(batch))
        .route("/~{user}/{repo}/info/lfs/objects/verify", post(verify))
        .route(
            "/~{user}/{repo}/info/lfs/objects/{oid}",
            get(download).put(upload),
        )
}

#[derive(Debug, Deserialize)]
struct BatchRequest {
    operation: String,
    transfers: Option<Vec<String>>,
    objects: Vec<LfsObjectSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LfsObjectSpec {
    oid: String,
    size: u64,
}

#[derive(Debug, Serialize)]
struct BatchResponse {
    transfer: String,
    objects: Vec<LfsObjectResponse>,
}

#[derive(Debug, Serialize)]
struct LfsObjectResponse {
    oid: String,
    size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<LfsActions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<LfsError>,
}

#[derive(Debug, Serialize)]
struct LfsActions {
    #[serde(skip_serializing_if = "Option::is_none")]
    download: Option<LfsLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upload: Option<LfsLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<LfsLink>,
}

#[derive(Debug, Serialize)]
struct LfsLink {
    href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    header: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,
}

#[derive(Debug, Serialize)]
struct LfsError {
    code: u16,
    message: String,
}

async fn batch(
    state: AppState,
    headers: HeaderMap,
    AxumPath((user, repo)): AxumPath<(String, String)>,
    Json(request): Json<BatchRequest>,
) -> Result<Response, AppError> {
    if !valid_user_repo(&user, &repo) {
        return Ok((StatusCode::BAD_REQUEST, "invalid user or repo").into_response());
    }

    if !authorize(&state, &headers, &user).await? {
        return Ok(unauthorized_response());
    }

    if let Some(transfers) = &request.transfers
        && !transfers.iter().any(|transfer| transfer == "basic")
    {
        return Ok((StatusCode::BAD_REQUEST, "unsupported transfer").into_response());
    }

    let operation = request.operation.as_str();
    if !matches!(operation, "download" | "upload") {
        return Ok((StatusCode::BAD_REQUEST, "unsupported operation").into_response());
    }

    // Extract auth header to pass through to action links
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            let mut h = HashMap::new();
            h.insert("Authorization".to_string(), s.to_string());
            h
        });

    let mut objects = Vec::with_capacity(request.objects.len());
    for object in request.objects {
        let Some(oid) = normalize_oid(&object.oid) else {
            objects.push(LfsObjectResponse {
                oid: object.oid,
                size: object.size,
                actions: None,
                error: Some(LfsError {
                    code: 422,
                    message: "invalid oid".to_string(),
                }),
            });
            continue;
        };

        let path = lfs_object_path(&state, &user, &repo, &oid);
        let exists = fs::try_exists(&path).await.unwrap_or(false);
        let response = match operation {
            "download" => {
                if exists {
                    LfsObjectResponse {
                        oid: oid.clone(),
                        size: object.size,
                        actions: Some(LfsActions {
                            download: Some(LfsLink {
                                href: lfs_object_href(
                                    &state.config.http.public_url,
                                    &user,
                                    &repo,
                                    &oid,
                                ),
                                header: auth_header.clone(),
                                expires_in: None,
                            }),
                            upload: None,
                            verify: None,
                        }),
                        error: None,
                    }
                } else {
                    LfsObjectResponse {
                        oid: oid.clone(),
                        size: object.size,
                        actions: None,
                        error: Some(LfsError {
                            code: 404,
                            message: "object not found".to_string(),
                        }),
                    }
                }
            }
            "upload" => {
                if exists {
                    LfsObjectResponse {
                        oid: oid.clone(),
                        size: object.size,
                        actions: None,
                        error: None,
                    }
                } else {
                    LfsObjectResponse {
                        oid: oid.clone(),
                        size: object.size,
                        actions: Some(LfsActions {
                            download: None,
                            upload: Some(LfsLink {
                                href: lfs_object_href(
                                    &state.config.http.public_url,
                                    &user,
                                    &repo,
                                    &oid,
                                ),
                                header: auth_header.clone(),
                                expires_in: None,
                            }),
                            verify: Some(LfsLink {
                                href: lfs_verify_href(&state.config.http.public_url, &user, &repo),
                                header: auth_header.clone(),
                                expires_in: None,
                            }),
                        }),
                        error: None,
                    }
                }
            }
            _ => unreachable!("validated operation"),
        };
        objects.push(response);
    }

    let response = BatchResponse {
        transfer: "basic".to_string(),
        objects,
    };

    Ok(json_response(StatusCode::OK, response))
}

async fn download(
    state: AppState,
    headers: HeaderMap,
    AxumPath((user, repo, oid)): AxumPath<(String, String, String)>,
) -> Result<Response, AppError> {
    if !valid_user_repo(&user, &repo) {
        return Ok((StatusCode::BAD_REQUEST, "invalid user or repo").into_response());
    }

    if !authorize(&state, &headers, &user).await? {
        return Ok(unauthorized_response());
    }

    let Some(oid) = normalize_oid(&oid) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid oid").into_response());
    };

    let path = lfs_object_path(&state, &user, &repo, &oid);
    let file = match fs::File::open(&path).await {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((StatusCode::NOT_FOUND, "object not found").into_response());
        }
        Err(err) => return Err(anyhow::Error::from(err).into()),
    };

    let size = file.metadata().await.map_err(anyhow::Error::from)?.len();
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&size.to_string()).unwrap(),
    );
    *response.status_mut() = StatusCode::OK;
    Ok(response)
}

async fn upload(
    state: AppState,
    headers: HeaderMap,
    AxumPath((user, repo, oid)): AxumPath<(String, String, String)>,
    body: Body,
) -> Result<Response, AppError> {
    if !valid_user_repo(&user, &repo) {
        return Ok((StatusCode::BAD_REQUEST, "invalid user or repo").into_response());
    }

    if !authorize(&state, &headers, &user).await? {
        return Ok(unauthorized_response());
    }

    let Some(oid) = normalize_oid(&oid) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid oid").into_response());
    };

    let path = lfs_object_path(&state, &user, &repo, &oid);
    if fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(StatusCode::OK.into_response());
    }

    let Some(parent) = path.parent() else {
        return Ok((StatusCode::BAD_REQUEST, "invalid oid path").into_response());
    };
    fs::create_dir_all(parent)
        .await
        .map_err(anyhow::Error::from)?;

    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .await
        .map_err(anyhow::Error::from)?;
    let mut stream = body.into_data_stream();
    let mut hasher = Sha256::new();
    while let Some(frame) = stream.next().await {
        let chunk = frame.map_err(anyhow::Error::from)?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(anyhow::Error::from)?;
    }
    file.flush().await.map_err(anyhow::Error::from)?;

    let computed = format!("{:x}", hasher.finalize());
    if computed != oid {
        let _ = fs::remove_file(&tmp_path).await;
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, "oid checksum mismatch").into_response());
    }
    fs::rename(&tmp_path, &path)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(StatusCode::OK.into_response())
}

async fn verify(
    state: AppState,
    headers: HeaderMap,
    AxumPath((user, repo)): AxumPath<(String, String)>,
    Json(request): Json<LfsObjectSpec>,
) -> Result<Response, AppError> {
    if !valid_user_repo(&user, &repo) {
        return Ok((StatusCode::BAD_REQUEST, "invalid user or repo").into_response());
    }

    if !authorize(&state, &headers, &user).await? {
        return Ok(unauthorized_response());
    }

    let Some(oid) = normalize_oid(&request.oid) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid oid").into_response());
    };

    let path = lfs_object_path(&state, &user, &repo, &oid);
    let metadata = match fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((StatusCode::NOT_FOUND, "object not found").into_response());
        }
        Err(err) => return Err(anyhow::Error::from(err).into()),
    };

    if metadata.len() != request.size {
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, "object size mismatch").into_response());
    }

    Ok(json_response(
        StatusCode::OK,
        LfsObjectSpec {
            oid: request.oid,
            size: request.size,
        },
    ))
}

fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response {
    let mut response = Json(body).into_response();
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(LFS_CONTENT_TYPE),
    );
    response
}

fn valid_user_repo(user: &str, repo: &str) -> bool {
    let user_ok = re!(r"^[a-zA-Z0-9]+$").is_match(user);
    let repo_ok = re!(r"^[.\-a-zA-Z0-9]+\.git$").is_match(repo);
    user_ok && repo_ok
}

fn normalize_oid(oid: &str) -> Option<String> {
    if !re!(r"^[0-9a-fA-F]{64}$").is_match(oid) {
        return None;
    }
    Some(oid.to_ascii_lowercase())
}

fn lfs_object_href(public_url: &str, user: &str, repo: &str, oid: &str) -> String {
    format!("{public_url}/~{user}/{repo}/info/lfs/objects/{oid}")
}

fn lfs_verify_href(public_url: &str, user: &str, repo: &str) -> String {
    format!("{public_url}/~{user}/{repo}/info/lfs/objects/verify")
}

fn lfs_object_path(state: &AppState, user: &str, repo: &str, oid: &str) -> PathBuf {
    let base = lfs_repo_path(state, user, repo);
    let (prefix, suffix) = oid.split_at(2);
    let (mid, _) = suffix.split_at(2);
    base.join("objects").join(prefix).join(mid).join(oid)
}

fn lfs_repo_path(state: &AppState, user: &str, repo: &str) -> PathBuf {
    state.config.git.lfs_path.join(user).join(repo)
}

fn unauthorized_response() -> Response {
    (StatusCode::UNAUTHORIZED, "lfs authentication required").into_response()
}

async fn authorize(state: &AppState, headers: &HeaderMap, user: &str) -> Result<bool, AppError> {
    let Some(header_value) = headers.get(header::AUTHORIZATION) else {
        return Ok(false);
    };

    let Ok(header_str) = header_value.to_str() else {
        return Ok(false);
    };

    let mut parts = header_str.split_whitespace();
    if parts.next() != Some("RemoteAuth") {
        return Ok(false);
    }

    let Some(token) = parts.next() else {
        return Ok(false);
    };

    let Some(record) = model::lfs::get_by_token_with_user(&state.db, token).await? else {
        return Ok(false);
    };

    if record.username != user {
        return Ok(false);
    }

    if record.expires <= OffsetDateTime::now_utc() {
        return Ok(false);
    }

    Ok(true)
}
