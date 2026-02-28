use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::AppState;
use axum::extract::{Query, State};
use axum::Json;
use redis::AsyncCommands;
use serde::Deserialize;

const CACHE_TTL_SECS: u64 = 3600; // 1 hour

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    /// Bias results toward this latitude
    pub near_lat: Option<f64>,
    /// Bias results toward this longitude
    pub near_lng: Option<f64>,
}

pub async fn search(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Query(params): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let q = params.q.trim().to_string();
    if q.is_empty() {
        return Ok(Json(serde_json::json!([])));
    }

    // Build Redis cache key
    let near_suffix = params
        .near_lat
        .zip(params.near_lng)
        .map(|(lat, lng)| format!(":{:.2}:{:.2}", lat, lng))
        .unwrap_or_default();
    let cache_key = format!("geocode:v1:{}{}", &q[..q.len().min(120)], near_suffix);

    // Try cache first
    let mut redis = state.redis.clone();
    if let Ok(cached) = redis.get::<_, String>(&cache_key).await {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cached) {
            return Ok(Json(val));
        }
    }

    // Build Nominatim query params
    let mut url_params: Vec<(&str, String)> = vec![
        ("q", q.clone()),
        ("format", "json".into()),
        ("limit", "6".into()),
        ("accept-language", "th,en".into()),
        ("addressdetails", "1".into()),
    ];

    if let (Some(lat), Some(lng)) = (params.near_lat, params.near_lng) {
        let d = 1.5_f64;
        url_params.push((
            "viewbox",
            format!("{},{},{},{}", lng - d, lat + d, lng + d, lat - d),
        ));
        url_params.push(("bounded", "0".into()));
    }

    let client = reqwest::Client::new();
    let res = client
        .get("https://nominatim.openstreetmap.org/search")
        .header("User-Agent", "GPS-Trip-Tracker/1.0")
        .query(&url_params)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("Nominatim request failed: {e}");
            AppError::Internal
        })?;

    let body = res
        .json::<serde_json::Value>()
        .await
        .map_err(|_| AppError::Internal)?;

    // Cache result for 1 hour
    let _ = redis
        .set_ex::<_, _, String>(&cache_key, body.to_string(), CACHE_TTL_SECS)
        .await;

    Ok(Json(body))
}
