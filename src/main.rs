mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod migrate;
mod models;
mod routes;
mod services;
mod ws;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = config::Config::from_env()?;
    let db = db::connect(&config.database_url).await?;
    let redis = db::connect_redis(&config.redis_url).await?;

    // Run migrations
    if let Err(e) = migrate::run_migrations(&config.database_url).await {
        tracing::warn!("Migration skipped: {e}. Run migrations manually.");
    }

    let app = Router::new()
        .merge(routes::api())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(AppState {
            db,
            redis,
            config: config.clone(),
            rooms: Arc::new(RwLock::new(HashMap::new())),
        });

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server listening on http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
    pub redis: redis::aio::ConnectionManager,
    pub config: config::Config,
    pub rooms: Arc<RwLock<HashMap<Uuid, broadcast::Sender<String>>>>,
}
