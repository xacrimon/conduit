use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::model::user::UserId;

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: UserId,
    pub expires: OffsetDateTime,
}

pub async fn create(db: &PgPool, user_id: UserId) -> Result<Session> {
    let buf: [u8; 16] = rand::random();
    let token = BASE64_STANDARD.encode(buf);
    let expires = OffsetDateTime::now_utc() + time::Duration::days(30);

    sqlx::query!(
        "INSERT INTO sessions (token, user_id, expires) VALUES ($1, $2, $3)",
        token,
        user_id.0,
        expires
    )
    .execute(db)
    .await?;

    Ok(Session {
        token,
        user_id,
        expires,
    })
}

pub async fn get_by_token(db: &PgPool, token: &str) -> Result<Option<Session>> {
    let record = sqlx::query!(
        "SELECT user_id, expires FROM sessions WHERE token = $1",
        token
    )
    .fetch_optional(db)
    .await?;

    if let Some(record) = record {
        Ok(Some(Session {
            token: token.to_owned(),
            user_id: UserId(record.user_id),
            expires: record.expires,
        }))
    } else {
        Ok(None)
    }
}
