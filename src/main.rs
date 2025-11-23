mod auth;
mod config;
mod db;
mod metrics;
mod model;
mod routes;
mod signal;
mod utils;

use anyhow::Result;
use axum::{Router, middleware};
use config::Config;
use tower::ServiceBuilder;
use tracing::{error, info};

const ADDR: &str = "0.0.0.0:8080";

#[derive(Clone)]
struct AppState {}

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(8)
        .build()
        .unwrap()
        .block_on(run())
}

async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .unwrap();

    let config = Config::load(None).await?;
    let (ct, tt) = signal::bind();
    db::get().await?;
    metrics::get();

    let app_state = AppState {};
    let middleware = ServiceBuilder::new().layer(middleware::from_fn(auth::middleware));

    let app = Router::new()
        .merge(routes::routes())
        .merge(metrics::routes())
        .layer(middleware)
        .with_state(app_state);

    {
        let signal = ct.cancelled_owned();
        tt.spawn(async move {
            let listener = tokio::net::TcpListener::bind(ADDR).await.unwrap();
            info!("http server worker starting on {}", ADDR);
            if let Err(err) = axum::serve(listener, app)
                .with_graceful_shutdown(signal)
                .await
            {
                error!("http server worker error: {}", err);
            }
        });
    }

    tt.wait().await;
    Ok(())
}
