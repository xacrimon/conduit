#![feature(cfg_select)]
#![feature(unsafe_pinned)]

mod config;
mod db;
mod jobs;
mod libssh;
mod metrics;
mod middleware;
mod model;
mod routes;
mod signal;
mod ssh;
mod state;
mod utils;

use anyhow::Result;
use axum::Router;
use config::Config;
use state::{AppState, AppStateInner};
use tokio::fs;
use tower::ServiceBuilder;
use tracing::{debug, error, info};

const VERSION: &str = env!("CONDUIT_VERSION");

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

    info!("conduit {}", VERSION);
    libssh::init();
    let config = Config::load(None).await?;
    let (cancel_token, task_tracker) = signal::bind();
    let db = db::connect(&config.database).await?;
    let state = AppState::new(AppStateInner {
        db,
        config,
        cancel_token,
        task_tracker,
    });

    metrics::get();

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
        .with_state(state.clone());

    {
        let signal = state.cancel_token.clone().cancelled_owned();
        let addr = format!("{}:{}", state.config.http.host, state.config.http.port);

        state.task_tracker.spawn(async move {
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
        let addr = state.config.ssh.host.clone();
        let state2 = state.clone();

        state.task_tracker.spawn(async move {
            let host_key = fs::read_to_string(&state2.config.ssh.host_key)
                .await
                .unwrap();

            let mut listener = libssh::Listener::bind(&host_key, &addr, state2.config.ssh.port)
                .await
                .unwrap();

            info!("ssh server worker starting on {}", addr);

            loop {
                tokio::select! {
                    _ = state2.cancel_token.cancelled() => break,
                    session = listener.accept() => {
                        let session = session.unwrap();
                        let state3 = state2.clone();

                        state2.task_tracker.spawn(async move {
                            debug!("accepted ssh connection");

                            if let Err(err) = ssh::handle_session(&state3, session).await {
                                error!("ssh session error: {}", err);
                            }
                        });
                    },
                }
            }
        });
    }

    state.task_tracker.wait().await;
    state.db.close().await;
    libssh::finalize();
    Ok(())
}
