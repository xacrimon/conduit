use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserId(pub(super) i32);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
}

pub async fn create(db: &PgPool, username: &str, email: &str, password: &str) -> Result<()> {
    let password_hash = hash_password(password);

    sqlx::query!(
        "INSERT INTO users (username, email, password_hash, created_at, display_name, biography) VALUES ($1, $2, $3, now(), $4, $5)",
        username,
        email,
        password_hash,
        username,
        "",
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

pub async fn get_id_by_username(db: &PgPool, username: &str) -> Result<Option<UserId>> {
    let record = sqlx::query_scalar!("SELECT id FROM users WHERE username = $1", username)
        .fetch_optional(db)
        .await?;

    Ok(record.map(UserId))
}

fn hash_password(password: &str) -> String {
    let password_hash_bytes = Sha256::digest(password.as_bytes());
    let password_hash = BASE64_STANDARD.encode(password_hash_bytes);
    password_hash
}

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub biography: String,
}

pub async fn get_profile(db: &PgPool, user_id: UserId) -> Result<Option<UserProfile>> {
    let record = sqlx::query!(
        "SELECT username, email, display_name, biography FROM users WHERE id = $1",
        user_id.0,
    )
    .fetch_optional(db)
    .await?;

    Ok(record.map(|r| UserProfile {
        username: r.username,
        email: r.email,
        display_name: r.display_name,
        biography: r.biography,
    }))
}

pub async fn update_profile(
    db: &PgPool,
    user_id: UserId,
    email: &str,
    display_name: &str,
    biography: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE users SET email = $1, display_name = $2, biography = $3 WHERE id = $4",
        email,
        display_name,
        biography,
        user_id.0,
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Load all SSH keys with their associated usernames.
/// Returns Vec<(encoded_key, username)> for authentication.
pub async fn get_all_ssh_keys(db: &PgPool) -> Result<Vec<(String, String)>> {
    let records = sqlx::query!(
        r#"
        SELECT uk.encoded, u.username
        FROM user_keys uk
        JOIN users u ON uk.user_id = u.id
        "#
    )
    .fetch_all(db)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| (r.encoded, r.username))
        .collect())
}

#[derive(Debug, Clone)]
pub struct UserKey {
    pub key_type: String,
    pub encoded: String,
    pub username: String,
    pub hostname: String,
    pub name: String,
}

/// Get all SSH keys for a specific user
pub async fn get_user_keys(db: &PgPool, user_id: UserId) -> Result<Vec<UserKey>> {
    let records = sqlx::query!(
        r#"
        SELECT type, encoded, username, hostname, name
        FROM user_keys
        WHERE user_id = $1
        ORDER BY name
        "#,
        user_id.0
    )
    .fetch_all(db)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| UserKey {
            key_type: r.r#type,
            encoded: r.encoded,
            username: r.username,
            hostname: r.hostname,
            name: r.name,
        })
        .collect())
}

/// Add a new SSH key for a user
pub async fn add_user_key(
    db: &PgPool,
    user_id: UserId,
    key_type: &str,
    encoded: &str,
    username: &str,
    hostname: &str,
    name: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO user_keys (type, encoded, username, hostname, user_id, name)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        key_type,
        encoded,
        username,
        hostname,
        user_id.0,
        name
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Delete an SSH key
pub async fn delete_user_key(db: &PgPool, key_type: &str, encoded: &str) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM user_keys
        WHERE type = $1 AND encoded = $2
        "#,
        key_type,
        encoded
    )
    .execute(db)
    .await?;

    Ok(())
}
