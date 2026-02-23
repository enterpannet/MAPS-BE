use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::gas_station;
use axum::extract::{Query, State};
use axum::Json;
use sea_orm::sea_query;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::AppState;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";
const RADIUS_KM: f64 = 100.0;

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
}

async fn fetch_from_overpass(south: f64, west: f64, north: f64, east: f64) -> Result<Vec<OverpassElement>, AppError> {
    let query = format!(
        r#"[out:json][timeout:25];(node["amenity"="fuel"]({},{},{},{});way["amenity"="fuel"]({},{},{},{}););out center;"#,
        south, west, north, east, south, west, north, east
    );
    let client = reqwest::Client::new();
    let res = client
        .post(OVERPASS_URL)
        .body(query)
        .send()
        .await
        .map_err(|_| AppError::Internal)?;
    if !res.status().is_success() {
        return Err(AppError::Internal);
    }
    let json: serde_json::Value = res.json().await.map_err(|_| AppError::Internal)?;
    let empty_arr: Vec<serde_json::Value> = Vec::new();
    let elements_arr = json["elements"].as_array().unwrap_or(&empty_arr);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for el in elements_arr {
        let (lat, lng) = if el["type"] == "node" {
            (
                el["lat"].as_f64().unwrap_or(0.0),
                el["lon"].as_f64().unwrap_or(0.0),
            )
        } else if let Some(center) = el.get("center") {
            (
                center["lat"].as_f64().unwrap_or(0.0),
                center["lon"].as_f64().unwrap_or(0.0),
            )
        } else {
            continue;
        };
        let key = format!("{:.5},{:.5}", lat, lng);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        let empty: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let tags = el.get("tags").and_then(|t| t.as_object()).unwrap_or(&empty);
        out.push(OverpassElement {
            osm_id: el["id"].as_i64().unwrap_or(0),
            osm_type: el["type"].as_str().unwrap_or("node").to_string(),
            lat,
            lng,
            name: tags.get("name").and_then(|v| v.as_str()).map(String::from),
            brand: tags.get("brand").and_then(|v| v.as_str()).map(String::from),
        });
    }
    Ok(out)
}

struct OverpassElement {
    osm_id: i64,
    osm_type: String,
    lat: f64,
    lng: f64,
    name: Option<String>,
    brand: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Query(q): Query<GasStationsQuery>,
) -> Result<Json<Vec<GasStationResponse>>, AppError> {
    let (south, west, north, east) = bbox_from_center(q.lat, q.lng);

    // 1. Query DB first
    let from_db = gas_station::Entity::find()
        .filter(gas_station::Column::Lat.gte(south))
        .filter(gas_station::Column::Lat.lte(north))
        .filter(gas_station::Column::Lng.gte(west))
        .filter(gas_station::Column::Lng.lte(east))
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let stations = if from_db.is_empty() {
        // 2. Fetch from Overpass and save to DB
        let elements = fetch_from_overpass(south, west, north, east).await?;
        let now = chrono::Utc::now();
        for el in &elements {
            let am = gas_station::ActiveModel {
                osm_id: Set(el.osm_id),
                osm_type: Set(el.osm_type.clone()),
                lat: Set(el.lat),
                lng: Set(el.lng),
                name: Set(el.name.clone()),
                brand: Set(el.brand.clone()),
                created_at: Set(now.into()),
                ..Default::default()
            };
            let _ = gas_station::Entity::insert(am)
                .on_conflict(
                    sea_query::OnConflict::columns([
                        gas_station::Column::OsmType,
                        gas_station::Column::OsmId,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec(&state.db)
                .await;
        }
        elements
            .into_iter()
            .map(|e| GasStationResponse {
                id: e.osm_id,
                lat: e.lat,
                lng: e.lng,
                name: e.name,
                brand: e.brand,
            })
            .collect()
    } else {
        from_db
            .into_iter()
            .map(|s| GasStationResponse {
                id: s.osm_id,
                lat: s.lat,
                lng: s.lng,
                name: s.name,
                brand: s.brand,
            })
            .collect()
    };

    Ok(Json(stations))
}
