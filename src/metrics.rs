use super::AppState;
use anyhow::Result;
use axum::response::Response;
use axum::routing;
use prometheus::Encoder;
use prometheus::TextEncoder;
use prometheus::{self, IntCounter, IntGauge, register_int_counter, register_int_gauge};
use std::sync::LazyLock;

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

    (hook: IntCounter, $($tail:tt)*) => { register_int_counter!($($tail)*) };
    (hook: IntGauge, $($tail:tt)*) => { register_int_gauge!($($tail)*) };
}

metrics! {
    IntCounter, test_counter, "A test counter metric",
    IntGauge, test_gauge, "A test gauge metric"
}

pub fn get() -> &'static Metrics {
    static METRICS: LazyLock<Metrics> = LazyLock::new(|| Metrics::register().unwrap());
    &METRICS
}

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/metrics", routing::get(handler))
}

async fn handler() -> Response {
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
