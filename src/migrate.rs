//! รัน migration จริงที่สตาร์ทแอป โดยอ่าน SQL จากไฟล์ที่ build script embed ไว้
//! เพิ่ม migration ใหม่: สร้างไฟล์ migrations/NNN_name.sql แล้ว build ใหม่ (ไม่ต้องแก้ไฟล์นี้)

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

pub async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await?;

    ensure_migrations_table(&pool).await?;

    for &(version, name, sql) in MIGRATIONS {
        if applied(&pool, version).await? {
            tracing::debug!("Migration {} already applied", name);
            continue;
        }
        tracing::info!("Running migration {} (version {})", name, version);
        run_sql(&pool, sql).await?;
        record_applied(&pool, version, name).await?;
    }

    Ok(())
}

async fn ensure_migrations_table(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _app_migrations (
            version BIGINT PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn applied(pool: &PgPool, version: i64) -> anyhow::Result<bool> {
    let opt: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM _app_migrations WHERE version = $1")
        .bind(version)
        .fetch_optional(pool)
        .await?;
    Ok(opt.is_some())
}

async fn record_applied(pool: &PgPool, version: i64, name: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO _app_migrations (version, name) VALUES ($1, $2)")
        .bind(version)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

/// รัน SQL แยกตาม statement (คั่นด้วย ; ตามด้วย newline)
async fn run_sql(pool: &PgPool, sql: &str) -> anyhow::Result<()> {
    let normalized = sql.replace("\r\n", "\n");
    let statements: Vec<&str> = normalized
        .split(";\n")
        .map(|s| s.trim())
        .filter(|s| {
            !s.is_empty()
                && !s.lines().all(|l| {
                    let t = l.trim();
                    t.is_empty() || t.starts_with("--")
                })
        })
        .collect();

    for stmt in statements {
        let s = if stmt.ends_with(';') {
            stmt.trim_end_matches(';').trim()
        } else {
            stmt
        };
        if s.is_empty() {
            continue;
        }
        sqlx::query(s).execute(pool).await?;
    }

    Ok(())
}
