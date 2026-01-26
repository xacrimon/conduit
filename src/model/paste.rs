use anyhow::Result;
use futures_util::FutureExt;
use sqlx::PgPool;

use crate::model::user::UserId;
use crate::{db, utils};

pub struct Paste {
    pub id: String,
    pub visibility: Visibility,
}

pub enum Visibility {
    Public,
    Unlisted,
    Private,
}

pub struct File {
    pub paste_id: i32,
    pub filename: String,
    pub content: String,
}

pub struct PasteInfo {
    pub id: String,
    pub visibility: String,
    pub filename: String,
}

pub struct PasteWithFile {
    pub id: String,
    pub user_id: UserId,
    pub visibility: String,
    pub filename: String,
    pub content: String,
}

pub async fn get_user_pastes(db: &PgPool, user_id: UserId) -> Result<Vec<PasteInfo>> {
    let pastes = sqlx::query_as!(
        PasteInfo,
        "SELECT p.id, p.visibility, pf.filename
         FROM pastes p
         JOIN paste_files pf ON p.id = pf.paste_id
         WHERE p.user_id = $1
         ORDER BY p.id DESC",
        user_id.0
    )
    .fetch_all(db)
    .await?;

    Ok(pastes)
}

pub async fn get_paste(db: &PgPool, paste_id: &str) -> Result<Option<PasteWithFile>> {
    let record = sqlx::query!(
        "SELECT p.id, p.user_id, p.visibility, pf.filename, pf.content
         FROM pastes p
         JOIN paste_files pf ON p.id = pf.paste_id
         WHERE p.id = $1",
        paste_id
    )
    .fetch_optional(db)
    .await?;

    Ok(record.map(|r| PasteWithFile {
        id: r.id,
        user_id: UserId(r.user_id),
        visibility: r.visibility,
        filename: r.filename,
        content: r.content,
    }))
}

pub async fn delete_paste(db: &PgPool, user_id: UserId, paste_id: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM pastes WHERE id = $1 AND user_id = $2",
        paste_id,
        user_id.0
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn create_paste(
    db: &PgPool,
    user_id: UserId,
    visibility: Visibility,
    filename: String,
    content: String,
) -> Result<String> {
    let visibility = match visibility {
        Visibility::Public => "public",
        Visibility::Unlisted => "unlisted",
        Visibility::Private => "private",
    };

    let id = db::transaction(
        db,
        (visibility, filename, content),
        |txn, (visibility, filename, content)| {
            async move {
                let id = utils::unique_string(txn, "pastes", "id", 4).await;

                sqlx::query!(
                    "INSERT INTO pastes (id, user_id, visibility) VALUES ($1, $2, $3)",
                    id,
                    user_id.0,
                    visibility
                )
                .execute(&mut **txn)
                .await?;

                sqlx::query!(
                    "INSERT INTO paste_files (paste_id, filename, content) VALUES ($1, $2, $3)",
                    id,
                    filename,
                    content
                )
                .execute(&mut **txn)
                .await?;

                Ok(id)
            }
            .boxed()
        },
    )
    .await?;

    Ok(id)
}
