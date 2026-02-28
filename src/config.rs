use anyhow::Result;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub upload_dir: std::path::PathBuf,
    pub nrel_api_key: Option<String>,
    pub ocm_api_key: Option<String>,
    pub tankerkoenig_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/maps".into()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production-32bytes!!".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3001),
            upload_dir: std::env::var("UPLOAD_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("./uploads")),
            nrel_api_key: std::env::var("NREL_API_KEY").ok(),
            ocm_api_key: std::env::var("OCM_API_KEY").ok(),
            tankerkoenig_api_key: std::env::var("TANKERKOENIG_API_KEY").ok(),
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
        })
    }
}
