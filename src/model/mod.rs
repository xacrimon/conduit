pub mod lfs;
pub mod paste;
pub mod repository;
pub mod session;
pub mod user;

use anyhow::Result;
use futures_util::future::BoxFuture;
use sqlx::{PgPool, PgTransaction};

use crate::db;

pub(crate) enum Target<'p> {
    Pool(&'p PgPool),
    Transaction(&'p mut sqlx::PgTransaction<'p>),
}

async fn atomic<A, T, F>(target: impl IntoTarget<'_>, args: A, mut callback: F) -> Result<T>
where
    for<'c> F: FnMut(&'c mut PgTransaction<'_>, &'c A) -> BoxFuture<'c, Result<T>>,
{
    match target.into() {
        Target::Pool(pool) => db::transaction(pool, args, callback).await,
        Target::Transaction(txn) => callback(txn, &args).await,
    }
}

impl<'p> From<&'p PgPool> for Target<'p> {
    fn from(pool: &'p PgPool) -> Self {
        Self::Pool(pool)
    }
}

impl<'p> From<&'p mut sqlx::PgTransaction<'p>> for Target<'p> {
    fn from(transaction: &'p mut sqlx::PgTransaction<'p>) -> Self {
        Self::Transaction(transaction)
    }
}

pub(crate) trait IntoTarget<'p>: Into<Target<'p>> {}

impl<'p, T> IntoTarget<'p> for T where T: Into<Target<'p>> {}
