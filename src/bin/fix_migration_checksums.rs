//! รัน: cargo run --bin fix_migration_checksums
//! แก้ไข checksum ใน _sqlx_migrations ให้ตรงกับไฟล์ migration ปัจจุบัน

use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = PgPool::connect(&url).await?;

    let updates = [
        (1, "d9673bcafb4543b0240cbc3e732e32bf17d0cf4f7674b8fe6a7a7164506815cd21024fb66b855b57087a9b4dd0981147"),
        (2, "d2f73d2110aea4cd9164cab0342a66847e030972445ba58ddaa2deb98f1e1066d6c92acfbaaff62451b8d987c3054ded"),
        (3, "468c31641835ea2b07da74a3644a5d2a89a9dabfb06aec0f79e1a65a2cd56e40ed5bbc3e03f398e4922e5d99845b9e83"),
        (4, "f1b7d9c0ae570e96bf6fd89815957b3a7bc0b8a41fae7bfb41dfa528d8c5f7d25af300551a09ad8fd51d2835ffc829f3"),
        (5, "65acb75c0e9ef848651c290c06df7d057e35c103c1047d86d0c4ae8dab17e9a66e4102e3d680bb63d38e34728cbcee50"),
        (6, "31741ef6811bcce50cca2b40667a1039930b0881d112b1c9a78a0b1e7e6277f0bc296e938756975017f7b44bc8181a8b"),
    ];

    for (version, hex) in updates {
        sqlx::query("UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') WHERE version = $2")
            .bind(hex)
            .bind(version as i64)
            .execute(&pool)
            .await?;
        println!("Updated checksum for migration {}", version);
    }

    println!("Done. Run: sqlx migrate run");
    Ok(())
}
