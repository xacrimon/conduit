use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::FutureExt;
use time::OffsetDateTime;

use super::{IntoTarget, atomic};
use crate::model::user::UserId;

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: UserId,
    pub expires: OffsetDateTime,
}

pub async fn create(db: impl IntoTarget<'_>, user_id: UserId) -> Result<Session> {
    let (token, expires) = atomic(db, (), |txn, _| {
        let buf: [u8; 16] = rand::random();
        let token = BASE64_STANDARD.encode(buf);
        let expires = OffsetDateTime::now_utc() + time::Duration::days(30);

        async move {
            sqlx::query!(
                "INSERT INTO sessions (token, user_id, expires) VALUES ($1, $2, $3)",
                token,
                user_id.0,
                expires
            )
            .execute(&mut **txn)
            .await?;

            Ok((token, expires))
        }
        .boxed()
    })
    .await?;

    Ok(Session {
        token,
        user_id,
        expires,
    })
}

pub async fn get_by_token(db: impl IntoTarget<'_>, token: &str) -> Result<Option<Session>> {
    atomic(db, token, |txn, token| {
        async move {
            let record = sqlx::query!(
                "SELECT user_id, expires FROM sessions WHERE token = $1",
                token
            )
            .fetch_optional(&mut **txn)
            .await?;

            Ok(record.map(|record| Session {
                token: token.to_string(),
                user_id: UserId(record.user_id),
                expires: record.expires,
            }))
        }
        .boxed()
    })
    .await
}

#[derive(Debug, Clone)]
pub struct SessionWithUser {
    pub token: String,
    pub user_id: UserId,
    pub username: String,
    pub expires: OffsetDateTime,
}

/// Get session with user data in a single query
pub async fn get_by_token_with_user(
    db: impl IntoTarget<'_>,
    token: &str,
) -> Result<Option<SessionWithUser>> {
    atomic(db, token, |txn, token| {
        async move {
            let record = sqlx::query!(
                r#"
                SELECT s.user_id, s.expires, u.username
                FROM sessions s
                JOIN users u ON s.user_id = u.id
                WHERE s.token = $1
                "#,
                token
            )
            .fetch_optional(&mut **txn)
            .await?;

            Ok(record.map(|record| SessionWithUser {
                token: token.to_string(),
                user_id: UserId(record.user_id),
                username: record.username,
                expires: record.expires,
            }))
        }
        .boxed()
    })
    .await
}
