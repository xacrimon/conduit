use anyhow::Result;
use futures_util::FutureExt;
use time::OffsetDateTime;

use super::{IntoTarget, atomic};
use crate::model::user::UserId;

pub struct Repository {
    pub id: i32,
    pub user_id: UserId,
    pub name: String,
    pub description: String,
    pub visibility: String,
    pub default_branch: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct RepositoryInfo {
    pub name: String,
    pub description: String,
    pub visibility: String,
}

pub async fn create(
    db: impl IntoTarget<'_>,
    user_id: UserId,
    name: &str,
    description: &str,
    visibility: &str,
) -> Result<i32> {
    atomic(
        db,
        (name, description, visibility),
        |txn, (name, description, visibility)| {
            async move {
                let id = sqlx::query_scalar!(
                    "INSERT INTO repositories (user_id, name, description, visibility)
                     VALUES ($1, $2, $3, $4)
                     RETURNING id",
                    user_id.0,
                    name,
                    description,
                    visibility
                )
                .fetch_one(&mut **txn)
                .await?;

                Ok(id)
            }
            .boxed()
        },
    )
    .await
}

pub async fn get_user_repositories(
    db: impl IntoTarget<'_>,
    user_id: UserId,
) -> Result<Vec<RepositoryInfo>> {
    atomic(db, (), |txn, _| {
        async move {
            let repos = sqlx::query_as!(
                RepositoryInfo,
                "SELECT name, description, visibility
                 FROM repositories
                 WHERE user_id = $1
                 ORDER BY name ASC",
                user_id.0
            )
            .fetch_all(&mut **txn)
            .await?;

            Ok(repos)
        }
        .boxed()
    })
    .await
}

pub async fn get_by_owner_and_name(
    db: impl IntoTarget<'_>,
    user_id: UserId,
    name: &str,
) -> Result<Option<Repository>> {
    atomic(db, name, |txn, name| {
        async move {
            let record = sqlx::query!(
                "SELECT id, user_id, name, description, visibility, default_branch, created_at, updated_at
                 FROM repositories
                 WHERE user_id = $1 AND name = $2",
                user_id.0,
                name
            )
            .fetch_optional(&mut **txn)
            .await?;

            Ok(record.map(|r| Repository {
                id: r.id,
                user_id: UserId(r.user_id),
                name: r.name,
                description: r.description,
                visibility: r.visibility,
                default_branch: r.default_branch,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }))
        }
        .boxed()
    })
    .await
}

pub async fn update(
    db: impl IntoTarget<'_>,
    id: i32,
    user_id: UserId,
    description: &str,
    visibility: &str,
    default_branch: &str,
) -> Result<()> {
    atomic(
        db,
        (description, visibility, default_branch),
        |txn, (description, visibility, default_branch)| {
            async move {
                sqlx::query!(
                    "UPDATE repositories
                     SET description = $1, visibility = $2, default_branch = $3, updated_at = now()
                     WHERE id = $4 AND user_id = $5",
                    description,
                    visibility,
                    default_branch,
                    id,
                    user_id.0
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

pub async fn delete(db: impl IntoTarget<'_>, id: i32, user_id: UserId) -> Result<()> {
    atomic(db, (), |txn, _| {
        async move {
            sqlx::query!(
                "DELETE FROM repositories WHERE id = $1 AND user_id = $2",
                id,
                user_id.0
            )
            .execute(&mut **txn)
            .await?;

            Ok(())
        }
        .boxed()
    })
    .await
}
