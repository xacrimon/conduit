use std::time::Duration;

use anyhow::Result;
use base64::engine::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
use futures_util::FutureExt;
use time::OffsetDateTime;

use super::{IntoTarget, atomic};
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

pub async fn create(db: impl IntoTarget<'_>, user_id: UserId, ttl: Duration) -> Result<LfsToken> {
    atomic(db, (), |txn, _| {
        let buf: [u8; 16] = rand::random();
        let token = BASE64_URL_SAFE_NO_PAD.encode(buf);
        let expires = OffsetDateTime::now_utc() + ttl;

        async move {
            sqlx::query!(
                "INSERT INTO lfs_tokens (token, user_id, expires) VALUES ($1, $2, $3)",
                token,
                user_id.0,
                expires
            )
            .execute(&mut **txn)
            .await?;

            Ok(LfsToken {
                token,
                user_id,
                expires,
            })
        }
        .boxed()
    })
    .await
}

pub async fn get_by_token_with_user(
    db: impl IntoTarget<'_>,
    token: &str,
) -> Result<Option<LfsTokenWithUser>> {
    atomic(db, token, |txn, token| {
        async move {
            let record = sqlx::query!(
                r#"
                SELECT lt.user_id, lt.expires, u.username
                FROM lfs_tokens lt
                JOIN users u ON lt.user_id = u.id
                WHERE lt.token = $1
                "#,
                token
            )
            .fetch_optional(&mut **txn)
            .await?;

            Ok(record.map(|record| LfsTokenWithUser {
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
