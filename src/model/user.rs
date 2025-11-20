use crate::db;
use anyhow::Result;
use base64::engine::{Engine, general_purpose::STANDARD as BASE64_STANDARD};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy)]
pub struct UserId(pub(super) i32);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
}

pub async fn create(username: &str, password: &str) -> Result<()> {
    let db = db::get().await?;
    let password_hash_bytes = Sha256::digest(password.as_bytes());
    let password_hash = BASE64_STANDARD.encode(password_hash_bytes);

    sqlx::query!(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2)",
        username,
        password_hash,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn login(username: &str, password: &str) -> Result<UserId> {
    let db = db::get().await?;
    let password_hash_bytes = Sha256::digest(password.as_bytes());
    let password_hash = BASE64_STANDARD.encode(password_hash_bytes);

    let id = sqlx::query_scalar!(
        "SELECT id FROM users WHERE username = $1 AND password_hash = $2",
        username,
        password_hash,
    )
    .fetch_one(db)
    .await?;

    Ok(UserId(id))
}

pub async fn get_by_id(user_id: UserId) -> Result<Option<User>> {
    let db = db::get().await?;

    let record = sqlx::query!(
        "SELECT id, username, password_hash FROM users WHERE id = $1",
        user_id.0,
    )
    .fetch_optional(db)
    .await?;

    if let Some(record) = record {
        Ok(Some(User {
            id: UserId(record.id),
            username: record.username,
            password_hash: record.password_hash,
        }))
    } else {
        Ok(None)
    }
}
