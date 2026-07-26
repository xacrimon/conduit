use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::FutureExt;
use sha2::{Digest, Sha256};

use super::{IntoTarget, atomic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserId(pub(super) i32);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
}

pub async fn create(
    db: impl IntoTarget<'_>,
    username: &str,
    email: &str,
    password: &str,
) -> Result<()> {
    let password_hash = hash_password(password);

    atomic(
        db,
        (username, email, password_hash),
        |txn, (username, email, password_hash)| {
            async move {
                sqlx::query!(
                    "INSERT INTO users (username, email, password_hash, created_at, display_name, biography) VALUES ($1, $2, $3, now(), $4, $5)",
                    username,
                    email,
                    password_hash,
                    username,
                    "",
                )
                .execute(&mut **txn)
                .await?;

                Ok(())
            }
            .boxed()
        },
    )
    .await
}

pub async fn login(db: impl IntoTarget<'_>, username: &str, password: &str) -> Result<UserId> {
    let password_hash = hash_password(password);

    atomic(
        db,
        (username, password_hash),
        |txn, (username, password_hash)| {
            async move {
                let id = sqlx::query_scalar!(
                    "SELECT id FROM users WHERE username = $1 AND password_hash = $2",
                    username,
                    password_hash,
                )
                .fetch_one(&mut **txn)
                .await?;

                Ok(UserId(id))
            }
            .boxed()
        },
    )
    .await
}

pub async fn get_by_id(db: impl IntoTarget<'_>, user_id: UserId) -> Result<Option<User>> {
    atomic(db, (), |txn, _| {
        async move {
            let record = sqlx::query!(
                "SELECT id, username, password_hash FROM users WHERE id = $1",
                user_id.0,
            )
            .fetch_optional(&mut **txn)
            .await?;

            Ok(record.map(|record| User {
                id: UserId(record.id),
                username: record.username,
                password_hash: record.password_hash,
            }))
        }
        .boxed()
    })
    .await
}

pub async fn get_id_by_username(db: impl IntoTarget<'_>, username: &str) -> Result<Option<UserId>> {
    atomic(db, username, |txn, username| {
        async move {
            let record = sqlx::query_scalar!("SELECT id FROM users WHERE username = $1", username)
                .fetch_optional(&mut **txn)
                .await?;

            Ok(record.map(UserId))
        }
        .boxed()
    })
    .await
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

pub async fn get_profile(db: impl IntoTarget<'_>, user_id: UserId) -> Result<Option<UserProfile>> {
    atomic(db, (), |txn, _| {
        async move {
            let record = sqlx::query!(
                "SELECT username, email, display_name, biography FROM users WHERE id = $1",
                user_id.0,
            )
            .fetch_optional(&mut **txn)
            .await?;

            Ok(record.map(|r| UserProfile {
                username: r.username,
                email: r.email,
                display_name: r.display_name,
                biography: r.biography,
            }))
        }
        .boxed()
    })
    .await
}

pub async fn update_profile(
    db: impl IntoTarget<'_>,
    user_id: UserId,
    email: &str,
    display_name: &str,
    biography: &str,
) -> Result<()> {
    atomic(
        db,
        (email, display_name, biography),
        |txn, (email, display_name, biography)| {
            async move {
                sqlx::query!(
                    "UPDATE users SET email = $1, display_name = $2, biography = $3 WHERE id = $4",
                    email,
                    display_name,
                    biography,
                    user_id.0,
                )
                .execute(&mut **txn)
                .await?;

                Ok(())
            }
            .boxed()
        },
    )
    .await
}

/// Load all SSH keys with their associated usernames.
/// Returns Vec<(encoded_key, username)> for authentication.
pub async fn get_all_ssh_keys(db: impl IntoTarget<'_>) -> Result<Vec<(String, String)>> {
    atomic(db, (), |txn, _| {
        async move {
            let records = sqlx::query!(
                r#"
                SELECT uk.encoded, u.username
                FROM user_keys uk
                JOIN users u ON uk.user_id = u.id
                "#
            )
            .fetch_all(&mut **txn)
            .await?;

            Ok(records
                .into_iter()
                .map(|r| (r.encoded, r.username))
                .collect())
        }
        .boxed()
    })
    .await
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
pub async fn get_user_keys(db: impl IntoTarget<'_>, user_id: UserId) -> Result<Vec<UserKey>> {
    atomic(db, (), |txn, _| {
        async move {
            let records = sqlx::query!(
                r#"
                SELECT type, encoded, username, hostname, name
                FROM user_keys
                WHERE user_id = $1
                ORDER BY name
                "#,
                user_id.0
            )
            .fetch_all(&mut **txn)
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
        .boxed()
    })
    .await
}

/// Add a new SSH key for a user
pub async fn add_user_key(
    db: impl IntoTarget<'_>,
    user_id: UserId,
    key_type: &str,
    encoded: &str,
    username: &str,
    hostname: &str,
    name: &str,
) -> Result<()> {
    atomic(
        db,
        (key_type, encoded, username, hostname, name),
        |txn, (key_type, encoded, username, hostname, name)| {
            async move {
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
                .execute(&mut **txn)
                .await?;

                Ok(())
            }
            .boxed()
        },
    )
    .await
}

/// Delete an SSH key
pub async fn delete_user_key(db: impl IntoTarget<'_>, key_type: &str, encoded: &str) -> Result<()> {
    atomic(db, (key_type, encoded), |txn, (key_type, encoded)| {
        async move {
            sqlx::query!(
                r#"
                    DELETE FROM user_keys
                    WHERE type = $1 AND encoded = $2
                    "#,
                key_type,
                encoded
            )
            .execute(&mut **txn)
            .await?;

            Ok(())
        }
        .boxed()
    })
    .await
}
