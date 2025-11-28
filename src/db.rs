use std::env;
use std::time::Duration;

use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use log::LevelFilter;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, Executor, PgPool, PgTransaction};
use tokio::time;

use crate::config;

pub async fn connect(config: &config::Database) -> Result<PgPool> {
    let url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => format!(
            "postgres://{}:{}@{}:{}/{}",
            config.user, config.password, config.host, config.port, config.database
        ),
    };

    let options = PgPoolOptions::new()
        .max_connections(8)
        .min_connections(2)
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
        });

    let pool = options.connect(&url).await?;
    Ok(pool)
}

pub async fn transaction<A, T, F>(db: &PgPool, args: A, mut callback: F) -> Result<T>
where
    for<'c> F: FnMut(&'c mut PgTransaction<'_>, &'c A) -> BoxFuture<'c, Result<T>>,
{
    let mut backoff = Backoff::new();

    loop {
        let mut conn = db.acquire().await?;
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

        let steps = [10, 30, 90, 270, 810];
        if self.tries > steps.len() {
            return false;
        }

        let dur = Duration::from_millis(steps[self.tries - 1]);
        time::sleep(dur).await;

        true
    }
}
