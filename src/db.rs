use anyhow::Result;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use redis::aio::ConnectionManager;

pub async fn connect(database_url: &str) -> Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(10).min_connections(2);
    Ok(Database::connect(opt).await?)
}

pub async fn connect_redis(redis_url: &str) -> Result<ConnectionManager> {
    let client = redis::Client::open(redis_url)?;
    Ok(ConnectionManager::new(client).await?)
}
