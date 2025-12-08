mod config;
mod db;
mod libssh;
mod metrics;
mod middleware;
mod model;
mod routes;
mod signal;
mod utils;

use anyhow::Result;
use axum::Router;
use config::Config;
use sqlx::PgPool;
use tokio::fs;
use tower::ServiceBuilder;
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

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

    libssh::init();
    let config = Config::load(None).await?;
    let (ct, tt) = signal::bind();
    let db = db::connect(&config.database).await?;
    metrics::get();

    let state = AppState { db };
    let middleware = ServiceBuilder::new()
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::middleware,
        ))
        .layer(middleware::panic::middleware());

    let app = Router::new()
        .merge(routes::routes())
        .merge(metrics::routes())
        .layer(middleware)
        .with_state(state);

    {
        let signal = ct.clone().cancelled_owned();
        let addr = format!("{}:{}", config.http.host, config.http.port);

        tt.spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            info!("http server worker starting on {}", addr);
            if let Err(err) = axum::serve(listener, app)
                .with_graceful_shutdown(signal)
                .await
            {
                error!("http server worker error: {}", err);
            }
        });
    }

    {
        let ct = ct.clone();
        let addr = config.ssh.host.clone();
        let tt_clone = tt.clone();

        tt.spawn(async move {
            let host_key = fs::read_to_string(config.ssh.host_key).await.unwrap();
            let mut listener = libssh::Listener::bind(&host_key, &addr, config.ssh.port)
                .await
                .unwrap();
            info!("ssh server worker starting on {}", addr);

            loop {
                tokio::select! {
                    _ = ct.cancelled() => break,
                    session = listener.accept() => {
                        let mut session = session.unwrap();

                        tt_clone.spawn(async move {
                            info!("accepted ssh connection");
                            session.configure();
                            session.handle_key_exchange().await.unwrap();
                        });
                    },
                }
            }
        });
    }

    tt.wait().await;
    libssh::finalize();
    Ok(())
}
