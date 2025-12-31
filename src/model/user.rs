use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy)]
pub struct UserId(pub(super) i32);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
}

pub async fn create(db: &PgPool, username: &str, password: &str) -> Result<()> {
    let password_hash = hash_password(password);

    sqlx::query!(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2)",
        username,
        password_hash,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn login(db: &PgPool, username: &str, password: &str) -> Result<UserId> {
    let password_hash = hash_password(password);

    let id = sqlx::query_scalar!(
        "SELECT id FROM users WHERE username = $1 AND password_hash = $2",
        username,
        password_hash,
    )
    .fetch_one(db)
    .await?;

    Ok(UserId(id))
}

pub async fn get_by_id(db: &PgPool, user_id: UserId) -> Result<Option<User>> {
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

fn hash_password(password: &str) -> String {
    let password_hash_bytes = Sha256::digest(password.as_bytes());
    let password_hash = BASE64_STANDARD.encode(password_hash_bytes);
    password_hash
}
