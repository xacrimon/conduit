use anyhow::Result;
use futures::FutureExt;

use crate::model::user;
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

pub async fn create_paste(
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
