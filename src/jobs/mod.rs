mod lfs_tokens;
mod web_sessions;

use std::time::Duration;

use ::time::OffsetDateTime;
use anyhow::Result;
use futures_util::future::BoxFuture;
use tokio::time::{self, Interval};
use tracing::error;

use crate::state::AppState;

const CHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);
static JOBS: &[Job] = &[lfs_tokens::JOB, web_sessions::JOB];

struct Job {
    name: &'static str,
    interval: u64,
    run: fn(&AppState) -> BoxFuture<'_, Result<()>>,
}

pub struct Scheduler {
    interval: Interval,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut interval = time::interval(CHECK_INTERVAL);
        interval.reset();

        Self { interval }
    }

    pub async fn run(&mut self, state: &AppState) -> Result<()> {
        self.interval.tick().await;
        let now = OffsetDateTime::now_utc();

        for job in JOBS {
            let last_run = sqlx::query_scalar!(
                "SELECT last_run FROM jobs_last_run WHERE name = $1",
                job.name
            )
            .fetch_optional(&state.db)
            .await?;

            let should_run = match last_run {
                Some(timestamp) => {
                    let elapsed = (now - timestamp).whole_seconds();
                    assert!(elapsed >= 0);
                    elapsed as u64 >= job.interval
                }
                None => true,
            };

            if should_run {
                tokio::spawn(run_job(job, state.clone()));
            }
        }

        Ok(())
    }
}

async fn run_job(job: &Job, state: AppState) {
    if let Err(err) = (job.run)(&state).await {
        error!("failed to run job \"{}\": {:?}", job.name, err);
        return;
    }

    let res = sqlx::query!(
        "INSERT INTO jobs_last_run (name, last_run) VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE SET last_run = EXCLUDED.last_run",
        job.name,
        OffsetDateTime::now_utc()
    )
    .execute(&state.db)
    .await;

    if let Err(err) = res {
        error!(
            "failed to update job metadata after running job \"{}\": {:?}",
            job.name, err
        );
    }
}
