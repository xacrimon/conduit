use anyhow::{Error, Result};
use futures::FutureExt;
use futures::future::BoxFuture;
use log::LevelFilter;
use sqlx::Connection;
use sqlx::Executor;
use sqlx::Transaction;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres};
use std::env;
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio::time;

pub async fn get() -> Result<&'static PgPool> {
    static DB: OnceCell<PgPool> = OnceCell::const_new();

    DB.get_or_try_init::<Error, _, _>(async || {
        let url = env::var("DATABASE_URL").unwrap();
        let pool = pool_options().connect(&url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(pool)
    })
    .await
}

fn pool_options() -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(8)
        .min_connections(4)
        .acquire_slow_level(LevelFilter::Warn)
        .acquire_slow_threshold(Duration::from_millis(250))
        .acquire_timeout(Duration::from_secs(5))
        .max_lifetime(Duration::from_secs(3600))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|conn, _meta| {
            async move {
                conn.execute("SET application_name = 'conduit';").await?;
                Ok(())
            }
            .boxed()
        })
}

pub async fn transaction<A, T, F>(args: A, mut callback: F) -> Result<T>
where
    for<'c> F: FnMut(&'c mut Transaction<'_, Postgres>, &'c A) -> BoxFuture<'c, Result<T>>,
{
    let mut backoff = Backoff::new();

    loop {
        let mut conn = get().await?.acquire().await?;
        let mut txn = conn.begin().await?;

        match callback(&mut txn, &args).await {
            Ok(ret) => {
                txn.commit().await?;
                break Ok(ret);
            }
            Err(err) => {
                txn.rollback().await?;
                drop(conn);

                if let Some(sql_err) = err.downcast_ref::<sqlx::Error>()
                    && should_retry(sql_err)
                    && backoff.wait().await
                {
                    continue;
                }

                break Err(err);
            }
        }
    }
}

fn should_retry(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err
        && let Some(code) = db_err.code()
    {
        // https://www.postgresql.org/docs/17/errcodes-appendix.html
        matches!(code.as_ref(), "40001" | "40P01" | "23505" | "23P01")
    } else {
        false
    }
}

struct Backoff {
    tries: usize,
}

impl Backoff {
    fn new() -> Self {
        Self { tries: 0 }
    }

    async fn wait(&mut self) -> bool {
        self.tries += 1;

        if self.tries >= 5 {
            return false;
        }

        let steps = [10, 30, 90, 270, 810];
        let dur = Duration::from_millis(steps[self.tries - 1]);
        time::sleep(dur).await;

        true
    }
}
