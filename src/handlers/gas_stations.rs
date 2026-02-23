use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::gas_station;
use axum::extract::{Query, State};
use axum::Json;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};
use tracing;

use crate::AppState;

const DEFAULT_RADIUS_KM: f64 = 30.0;
const DEFAULT_MAX_STATIONS: u64 = 100;

/// ประมาณ 1 องศา latitude ≈ 111 km
fn bbox_from_center(lat: f64, lng: f64, radius_km: f64) -> (f64, f64, f64, f64) {
    let delta_lat = radius_km / 111.0;
    let delta_lng = radius_km / (111.0 * (lat.to_radians().cos()).max(0.01));
    let south = lat - delta_lat;
    let north = lat + delta_lat;
    let west = lng - delta_lng;
    let east = lng + delta_lng;
    (south, west, north, east)
}

#[derive(Deserialize)]
pub struct GasStationsQuery {
    pub lat: f64,
    pub lng: f64,
    #[serde(default)]
    pub radius_km: Option<f64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Serialize)]
pub struct GasStationResponse {
    pub id: i64,
    pub lat: f64,
    pub lng: f64,
    pub name: Option<String>,
    pub brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// ดึงจาก DB เท่านั้น (sync job อัพเดทข้อมูลให้วันละ 1 รอบ)
pub async fn list(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Query(q): Query<GasStationsQuery>,
) -> Result<Json<Vec<GasStationResponse>>, AppError> {
    let radius_km = q
        .radius_km
        .filter(|r| (10.0..=100.0).contains(r))
        .unwrap_or(DEFAULT_RADIUS_KM);
    let max_stations = q
        .limit
        .filter(|l| (20..=200).contains(l))
        .unwrap_or(DEFAULT_MAX_STATIONS);

    let (south, west, north, east) = bbox_from_center(q.lat, q.lng, radius_km);

    let from_db = match gas_station::Entity::find()
        .filter(gas_station::Column::Lat.gte(south))
        .filter(gas_station::Column::Lat.lte(north))
        .filter(gas_station::Column::Lng.gte(west))
        .filter(gas_station::Column::Lng.lte(east))
        .limit(500)
        .all(&state.db)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("gas-stations list failed (run migrations?): {}", e);
            return Ok(Json(vec![]));
        }
    };

    let mut sorted: Vec<_> = from_db
        .into_iter()
        .map(|s| {
            let dist_sq = (s.lat - q.lat).powi(2) + (s.lng - q.lng).powi(2);
            (s, dist_sq)
        })
        .collect();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let nearest: Vec<_> = sorted
        .into_iter()
        .take(max_stations as usize)
        .map(|(s, _)| s)
        .collect();

    let stations = nearest
        .into_iter()
        .map(|s| GasStationResponse {
            id: s.id,
            lat: s.lat,
            lng: s.lng,
            name: s.name,
            brand: s.brand,
            source: Some(s.source),
        })
        .collect();

    Ok(Json(stations))
}
