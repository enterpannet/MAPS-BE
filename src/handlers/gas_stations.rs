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
const NREL_URL: &str = "https://developer.nrel.gov/api/alt-fuel-stations/v1/nearest.json";
const OCM_URL: &str = "https://api.openchargemap.io/v3/poi/";
const TANKERKOENIG_URL: &str = "https://creativecommons.tankerkoenig.de/json/list.php";
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

/// ตรวจสอบว่าอยู่ในสหรัฐฯ (โดยประมาณ)
fn is_in_us(lat: f64, lng: f64) -> bool {
    lat >= 24.0 && lat <= 50.0 && lng >= -125.0 && lng <= -66.0
}

/// ตรวจสอบว่าอยู่ในเยอรมนี (โดยประมาณ)
fn is_in_germany(lat: f64, lng: f64) -> bool {
    lat >= 47.0 && lat <= 55.0 && lng >= 6.0 && lng <= 15.0
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

struct GasStationInsert {
    source: String,
    external_id: String,
    lat: f64,
    lng: f64,
    name: Option<String>,
    brand: Option<String>,
    osm_id: Option<i64>,
    osm_type: Option<String>,
}

// --- OSM (Overpass) - ทั่วโลก ---
async fn fetch_osm(south: f64, west: f64, north: f64, east: f64) -> Result<Vec<GasStationInsert>, AppError> {
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
    let elements = json["elements"].as_array().unwrap_or(&empty_arr);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for el in elements {
        let (lat, lng) = if el["type"] == "node" {
            (el["lat"].as_f64().unwrap_or(0.0), el["lon"].as_f64().unwrap_or(0.0))
        } else if let Some(c) = el.get("center") {
            (c["lat"].as_f64().unwrap_or(0.0), c["lon"].as_f64().unwrap_or(0.0))
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
        let osm_id = el["id"].as_i64().unwrap_or(0);
        let osm_type = el["type"].as_str().unwrap_or("node").to_string();
        out.push(GasStationInsert {
            source: "osm".into(),
            external_id: format!("{}:{}", osm_type, osm_id),
            lat,
            lng,
            name: tags.get("name").and_then(|v| v.as_str()).map(String::from),
            brand: tags.get("brand").and_then(|v| v.as_str()).map(String::from),
            osm_id: Some(osm_id),
            osm_type: Some(osm_type),
        });
    }
    Ok(out)
}

// --- NREL Alternative Fuel - สหรัฐฯ ---
async fn fetch_nrel(lat: f64, lng: f64, api_key: &str) -> Result<Vec<GasStationInsert>, AppError> {
    let radius_miles = 62.0; // ~100 km
    let url = format!(
        "{}?latitude={}&longitude={}&radius={}&radius_type=radius&api_key={}&status=all",
        NREL_URL, lat, lng, radius_miles, api_key
    );
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await.map_err(|_| AppError::Internal)?;
    if !res.status().is_success() {
        return Ok(vec![]);
    }
    let json: serde_json::Value = res.json().await.map_err(|_| AppError::Internal)?;
    let empty: Vec<serde_json::Value> = Vec::new();
    let stations = json["fuel_stations"].as_array().unwrap_or(&empty);
    let mut out = Vec::new();
    for s in stations {
        let id = s["id"].as_i64().or_else(|| s["id"].as_str().and_then(|x| x.parse().ok())).unwrap_or(0);
        let lat = s["latitude"].as_f64().unwrap_or(0.0);
        let lng = s["longitude"].as_f64().unwrap_or(0.0);
        let name = s["station_name"].as_str().map(String::from);
        let brand = s.get("ev_network").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| s.get("cards_accepted").and_then(|v| v.as_str()).map(String::from));
        out.push(GasStationInsert {
            source: "nrel".into(),
            external_id: id.to_string(),
            lat,
            lng,
            name,
            brand,
            osm_id: None,
            osm_type: None,
        });
    }
    Ok(out)
}

// --- Open Charge Map - สถานีชาร์จ EV ทั่วโลก ---
async fn fetch_ocm(lat: f64, lng: f64, api_key: &str) -> Result<Vec<GasStationInsert>, AppError> {
    let url = format!(
        "{}?output=json&latitude={}&longitude={}&distance=100&distanceunit=KM&maxresults=500&key={}",
        OCM_URL, lat, lng, api_key
    );
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await.map_err(|_| AppError::Internal)?;
    if !res.status().is_success() {
        return Ok(vec![]);
    }
    let json: serde_json::Value = res.json().await.map_err(|_| AppError::Internal)?;
    let empty_arr: Vec<serde_json::Value> = Vec::new();
    let items = json
        .as_array()
        .or_else(|| json.get("Items").and_then(|v| v.as_array()))
        .unwrap_or(&empty_arr);
    let mut out = Vec::new();
    for item in items {
        let id = item["ID"].as_i64().or_else(|| item["ID"].as_u64().map(|x| x as i64)).unwrap_or(0);
        let addr = item.get("AddressInfo").and_then(|a| a.as_object());
        let lat = addr.and_then(|a| a["Latitude"].as_f64()).unwrap_or(0.0);
        let lng = addr.and_then(|a| a["Longitude"].as_f64()).unwrap_or(0.0);
        let title = item.get("AddressInfo").and_then(|a| a["Title"].as_str()).map(String::from);
        let operator = item.get("OperatorInfo").and_then(|o| o["Title"].as_str()).map(String::from);
        out.push(GasStationInsert {
            source: "ocm".into(),
            external_id: id.to_string(),
            lat,
            lng,
            name: title,
            brand: operator,
            osm_id: None,
            osm_type: None,
        });
    }
    Ok(out)
}

// --- Tankerkoenig - เยอรมนี ปั๊มน้ำมัน + ราคา ---
async fn fetch_tankerkoenig(lat: f64, lng: f64, api_key: &str) -> Result<Vec<GasStationInsert>, AppError> {
    let url = format!(
        "{}?lat={}&lng={}&rad=100&type=all&apikey={}",
        TANKERKOENIG_URL, lat, lng, api_key
    );
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await.map_err(|_| AppError::Internal)?;
    if !res.status().is_success() {
        return Ok(vec![]);
    }
    let json: serde_json::Value = res.json().await.map_err(|_| AppError::Internal)?;
    if json.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return Ok(vec![]);
    }
    let empty: Vec<serde_json::Value> = Vec::new();
    let stations = json["stations"].as_array().unwrap_or(&empty);
    let mut out = Vec::new();
    for s in stations {
        let id = s["id"].as_str().unwrap_or("").to_string();
        let lat = s["lat"].as_f64().unwrap_or(0.0);
        let lng = s["lng"].as_f64().unwrap_or(0.0);
        let name = s["name"].as_str().map(String::from);
        let brand = s["brand"].as_str().map(String::from);
        out.push(GasStationInsert {
            source: "tankerkoenig".into(),
            external_id: id.clone(),
            lat,
            lng,
            name,
            brand,
            osm_id: None,
            osm_type: None,
        });
    }
    Ok(out)
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
        // 2. Fetch from all sources in parallel
        let mut all: Vec<GasStationInsert> = Vec::new();

        // OSM - always
        match fetch_osm(south, west, north, east).await {
            Ok(v) => all.extend(v),
            Err(_) => {}
        }

        // NREL - US only
        if is_in_us(q.lat, q.lng) {
            if let Some(ref key) = state.config.nrel_api_key {
                if let Ok(v) = fetch_nrel(q.lat, q.lng, key).await {
                    all.extend(v);
                }
            }
        }

        // Open Charge Map - global
        if let Some(ref key) = state.config.ocm_api_key {
            if let Ok(v) = fetch_ocm(q.lat, q.lng, key).await {
                all.extend(v);
            }
        }

        // Tankerkoenig - Germany only
        if is_in_germany(q.lat, q.lng) {
            if let Some(ref key) = state.config.tankerkoenig_api_key {
                if let Ok(v) = fetch_tankerkoenig(q.lat, q.lng, key).await {
                    all.extend(v);
                }
            }
        }

        // 3. Save to DB
        let now = chrono::Utc::now();
        for g in &all {
            let am = gas_station::ActiveModel {
                source: Set(g.source.clone()),
                external_id: Set(Some(g.external_id.clone())),
                lat: Set(g.lat),
                lng: Set(g.lng),
                name: Set(g.name.clone()),
                brand: Set(g.brand.clone()),
                osm_id: Set(g.osm_id),
                osm_type: Set(g.osm_type.clone()),
                created_at: Set(now.into()),
                ..Default::default()
            };
            let _ = gas_station::Entity::insert(am)
                .on_conflict(
                    sea_query::OnConflict::columns([
                        gas_station::Column::Source,
                        gas_station::Column::ExternalId,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec(&state.db)
                .await;
        }

        // 4. Re-query from DB
        let from_db = gas_station::Entity::find()
            .filter(gas_station::Column::Lat.gte(south))
            .filter(gas_station::Column::Lat.lte(north))
            .filter(gas_station::Column::Lng.gte(west))
            .filter(gas_station::Column::Lng.lte(east))
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?;

        from_db
            .into_iter()
            .map(|s| GasStationResponse {
                id: s.id,
                lat: s.lat,
                lng: s.lng,
                name: s.name,
                brand: s.brand,
                source: Some(s.source),
            })
            .collect()
    } else {
        from_db
            .into_iter()
            .map(|s| GasStationResponse {
                id: s.id,
                lat: s.lat,
                lng: s.lng,
                name: s.name,
                brand: s.brand,
                source: Some(s.source),
            })
            .collect()
    };

    Ok(Json(stations))
}
