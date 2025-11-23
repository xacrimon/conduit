use std::sync::LazyLock;

use anyhow::Result;
use axum::response::Response;
use axum::{Router, routing};
use prometheus::{self, Encoder, IntGauge, TextEncoder, register_int_gauge};

use super::AppState;

macro_rules! metrics {
    ($($t:ident, $name:ident, $desc:expr),*) => {
        pub struct Metrics {
            $(pub $name: $t),*
        }

        impl Metrics {
            fn register() -> Result<Self> {
                Ok(Self {
                    $($name: metrics!(
                        hook: $t,
                        concat!("conduit_", stringify!($name)),
                        $desc
                    )?),*
                })
            }
        }
    };

    (hook: IntGauge, $($tail:tt)*) => { register_int_gauge!($($tail)*) };
}

metrics! {
    IntGauge, rt_num_alive_tasks, " ",
    IntGauge, rt_global_queue_depth, " ",
    IntGauge, rt_worker_total_busy_duration, " ",
    IntGauge, rt_worker_park_unpark_count, " ",
    IntGauge, rt_num_blocking_threads, " ",
    IntGauge, rt_num_idle_blocking_threads, " ",
    IntGauge, rt_worker_local_queue_depth, " ",
    IntGauge, rt_blocking_queue_depth, " ",
    IntGauge, rt_spawned_tasks_count, " ",
    IntGauge, rt_remote_schedule_count, " ",
    IntGauge, rt_budget_forced_yield_count, " ",
    IntGauge, rt_worker_noop_count, " ",
    IntGauge, rt_worker_poll_count, " ",
    IntGauge, rt_worker_local_schedule_count, " ",
    IntGauge, rt_io_driver_fd_registered_count, " ",
    IntGauge, rt_io_driver_fd_deregistered_count, " ",
    IntGauge, rt_io_driver_ready_count, " "
}

pub fn get() -> &'static Metrics {
    static METRICS: LazyLock<Metrics> = LazyLock::new(|| Metrics::register().unwrap());
    &METRICS
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/metrics", routing::get(handler))
}

async fn handler() -> Response {
    refresh();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let format = encoder.format_type().to_string();

    Response::builder()
        .header("Content-Type", format)
        .status(200)
        .body(buffer.into())
        .unwrap()
}

fn refresh() {
    let metrics = get();
    let rt_metrics = tokio::runtime::Handle::current().metrics();

    metrics
        .rt_num_alive_tasks
        .set(rt_metrics.num_alive_tasks() as i64);

    metrics
        .rt_global_queue_depth
        .set(rt_metrics.global_queue_depth() as i64);

    metrics
        .rt_worker_total_busy_duration
        .set(rt_metrics.worker_total_busy_duration(0).as_millis() as i64);

    metrics
        .rt_worker_park_unpark_count
        .set(rt_metrics.worker_park_unpark_count(0) as i64);

    metrics
        .rt_num_blocking_threads
        .set(rt_metrics.num_blocking_threads() as i64);

    metrics
        .rt_num_idle_blocking_threads
        .set(rt_metrics.num_idle_blocking_threads() as i64);

    metrics
        .rt_worker_local_queue_depth
        .set(rt_metrics.worker_local_queue_depth(0) as i64);

    metrics
        .rt_blocking_queue_depth
        .set(rt_metrics.blocking_queue_depth() as i64);

    metrics
        .rt_spawned_tasks_count
        .set(rt_metrics.spawned_tasks_count() as i64);

    metrics
        .rt_remote_schedule_count
        .set(rt_metrics.remote_schedule_count() as i64);

    metrics
        .rt_budget_forced_yield_count
        .set(rt_metrics.budget_forced_yield_count() as i64);

    metrics
        .rt_worker_noop_count
        .set(rt_metrics.worker_noop_count(0) as i64);

    metrics
        .rt_worker_poll_count
        .set(rt_metrics.worker_poll_count(0) as i64);

    metrics
        .rt_worker_local_schedule_count
        .set(rt_metrics.worker_local_schedule_count(0) as i64);

    metrics
        .rt_io_driver_fd_registered_count
        .set(rt_metrics.io_driver_fd_registered_count() as i64);

    metrics
        .rt_io_driver_fd_deregistered_count
        .set(rt_metrics.io_driver_fd_deregistered_count() as i64);

    metrics
        .rt_io_driver_ready_count
        .set(rt_metrics.io_driver_ready_count() as i64);
}
