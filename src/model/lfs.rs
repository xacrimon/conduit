use std::time::Duration;

use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
use rand::RngCore;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::model::user::UserId;

#[derive(Debug, Clone)]
pub struct LfsToken {
    pub token: String,
    pub user_id: UserId,
    pub expires: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct LfsTokenWithUser {
    pub token: String,
    pub user_id: UserId,
    pub username: String,
    pub expires: OffsetDateTime,
}

pub async fn create(db: &PgPool, user_id: UserId, ttl: Duration) -> Result<LfsToken> {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let token = BASE64_URL_SAFE_NO_PAD.encode(buf);
    let expires = OffsetDateTime::now_utc() + ttl;

    sqlx::query!(
        "INSERT INTO lfs_tokens (token, user_id, expires) VALUES ($1, $2, $3)",
        token,
        user_id.0,
        expires
    )
    .execute(db)
    .await?;

    Ok(LfsToken {
        token,
        user_id,
        expires,
    })
}

pub async fn get_by_token_with_user(db: &PgPool, token: &str) -> Result<Option<LfsTokenWithUser>> {
    let record = sqlx::query!(
        r#"
        SELECT lt.user_id, lt.expires, u.username
        FROM lfs_tokens lt
        JOIN users u ON lt.user_id = u.id
        WHERE lt.token = $1
        "#,
        token
    )
    .fetch_optional(db)
    .await?;

    Ok(record.map(|record| LfsTokenWithUser {
        token: token.to_owned(),
        user_id: UserId(record.user_id),
        username: record.username,
        expires: record.expires,
    }))
}
