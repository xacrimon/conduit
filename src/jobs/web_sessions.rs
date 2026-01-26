use anyhow::Result;

use super::Job;
use crate::state::AppState;

pub(super) const JOB: Job = Job {
    name: "expired_lfs_tokens_cleanup",
    interval: 24 * 60 * 60,
    run: |state| Box::pin(run(state)),
};

async fn run(state: &AppState) -> Result<()> {
    sqlx::query!("DELETE FROM lfs_tokens WHERE expires < now()")
        .execute(&state.db)
        .await?;

    Ok(())
}
