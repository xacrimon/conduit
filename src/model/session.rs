use crate::db;
use crate::model::user::UserId;
use anyhow::Result;
use base64::engine::{Engine, general_purpose::STANDARD as BASE64_STANDARD};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: UserId,
    pub expires: OffsetDateTime,
}

pub async fn create(user_id: UserId) -> Result<Session> {
    let db = db::get().await?;
    let buf: [u8; 32] = rand::random();
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
