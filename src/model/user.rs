use crate::db;
use anyhow::Result;
use base64::engine::{Engine, general_purpose::STANDARD as BASE64_STANDARD};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy)]
pub struct UserId(pub(super) i32);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
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
