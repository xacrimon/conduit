use std::sync::LazyLock;

use anyhow::Result;
use axum::response::Response;
use axum::{Router, routing};
use prometheus::{self, Encoder, IntGauge, TextEncoder, register_int_gauge};

use crate::state::AppState;

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
    IntGauge, rt_num_alive_tasks, "Current number of alive tasks in the runtime.",
    IntGauge, rt_global_queue_depth, "Number of tasks currently scheduled in the runtime's global queue.",
    IntGauge, rt_worker_total_busy_duration, "Amount of time the worker has been busy.",
    IntGauge, rt_worker_park_unpark_count, "Total number of times the worker has parked and unparked.",
    IntGauge, rt_num_blocking_threads, "The number of additional threads spawned by the runtime to handle blocking tasks.",
    IntGauge, rt_num_idle_blocking_threads, "The number of idle threads, which have been spawned by the runtime to handle blocking tasks.",
    IntGauge, rt_worker_local_queue_depth, "The number of tasks currently scheduled in the worker's local queue.",
    IntGauge, rt_blocking_queue_depth, "Number of tasks currently scheduled in the blocking thread pool.",
    IntGauge, rt_spawned_tasks_count, "Number of tasks spawned in this runtime since it was created.",
    IntGauge, rt_remote_schedule_count, "Number of tasks scheduled from outside the runtime.",
    IntGauge, rt_budget_forced_yield_count, "Number of times tht tasks have been forced to yield back to the scheduler after exhausting their task budgets.",
    IntGauge, rt_worker_noop_count, "Number of times the worker unparked but performed no work before parking again.",
    IntGauge, rt_worker_poll_count, "Number of tasks the worker has polled.",
    IntGauge, rt_worker_local_schedule_count, "Number of tasks scheduled from within the runtime on the given worker's local queue.",
    IntGauge, rt_io_driver_fd_registered_count, "Number of file descriptors that have been registered with the runtime's I/O driver.",
    IntGauge, rt_io_driver_fd_deregistered_count, "Number of file descriptors that have been deregistered by the runtime's I/O driver.",
    IntGauge, rt_io_driver_ready_count, "Number of ready events processed by the runtime's I/O driver."
}

pub fn get() -> &'static Metrics {
    static METRICS: LazyLock<Metrics> = LazyLock::new(|| Metrics::register().unwrap());
    &*METRICS
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
