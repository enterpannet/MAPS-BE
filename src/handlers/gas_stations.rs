use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::gas_station;
use axum::extract::{Query, State};
use axum::Json;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::AppState;

const RADIUS_KM: f64 = 100.0;
const MAX_STATIONS: u64 = 200;

/// ประมาณ 1 องศา latitude ≈ 111 km
fn bbox_from_center(lat: f64, lng: f64) -> (f64, f64, f64, f64) {
    let delta_lat = RADIUS_KM / 111.0;
    let delta_lng = RADIUS_KM / (111.0 * (lat.to_radians().cos()).max(0.01));
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
    let (south, west, north, east) = bbox_from_center(q.lat, q.lng);

    let from_db = gas_station::Entity::find()
        .filter(gas_station::Column::Lat.gte(south))
        .filter(gas_station::Column::Lat.lte(north))
        .filter(gas_station::Column::Lng.gte(west))
        .filter(gas_station::Column::Lng.lte(east))
        .limit(MAX_STATIONS)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let stations = from_db
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
