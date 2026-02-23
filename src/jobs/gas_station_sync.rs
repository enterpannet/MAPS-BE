//! Sync ปั๊มน้ำมันจาก OSM เข้า DB ทุกวัน
//! ครอบคลุม: ไทย ลาว มาเลเซีย จีน (ภาคใต้)
//! รันวันละ 1 รอบ เพื่อไม่ให้หนักเครื่อง และอัพเดทเฉพาะจุดที่เปลี่ยน

use sea_orm::sea_query;
use sea_orm::{EntityTrait, Set};
use thiserror::Error;
use tracing::{info, warn};

use crate::models::gas_station;

#[derive(Error, Debug)]
enum SyncError {
    #[error("HTTP request failed")]
    Http(#[from] reqwest::Error),
    #[error("Overpass API error")]
    OverpassError,
}

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";
/// หน่วงระหว่าง request เพื่อไม่ให้ Overpass rate limit (แนะนำ ~2 วินาที)
const DELAY_BETWEEN_TILES_MS: u64 = 2500;

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

/// Tiles สำหรับ ไทย ลาว มาเลเซีย จีนภาคใต้ (2.5° x 2.5° ต่อ tile)
fn region_tiles() -> Vec<(f64, f64, f64, f64)> {
    let mut tiles = Vec::new();
    // south, west, north, east (ละติจูด ลองจิจูด)
    let step = 2.5;
    // ไทย+ลาว+มาเล: 1°N ถึง 23°N, 97°E ถึง 110°E
    let mut lat = 1.0;
    while lat < 23.0 {
        let mut lng = 97.0;
        while lng < 110.0 {
            tiles.push((lat, lng, lat + step, lng + step));
            lng += step;
        }
        lat += step;
    }
    // จีนภาคใต้ (ยูนนาน กวางสี กวางตุ้ง): 18°N ถึง 26°N, 97°E ถึง 112°E
    let mut lat = 18.0;
    while lat < 26.0 {
        let mut lng = 97.0;
        while lng < 112.0 {
            let (s, w, n, e) = (lat, lng, lat + step, lng + step);
            if !tiles.iter().any(|(ts, tw, _tn, _te)| *ts == s && *tw == w) {
                tiles.push((s, w, n, e));
            }
            lng += step;
        }
        lat += step;
    }
    tiles.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    tiles.dedup();
    tiles
}

async fn fetch_osm(
    client: &reqwest::Client,
    south: f64,
    west: f64,
    north: f64,
    east: f64,
) -> Result<Vec<GasStationInsert>, SyncError> {
    let query = format!(
        r#"[out:json][timeout:25];(node["amenity"="fuel"]({},{},{},{});way["amenity"="fuel"]({},{},{},{}););out center;"#,
        south, west, north, east, south, west, north, east
    );
    let res = client
        .post(OVERPASS_URL)
        .body(query)
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(SyncError::OverpassError);
    }
    let json: serde_json::Value = res.json().await?;
    let empty_arr: Vec<serde_json::Value> = Vec::new();
    let elements = json["elements"].as_array().unwrap_or(&empty_arr);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for el in elements {
        let (lat, lng) = if el["type"] == "node" {
            (
                el["lat"].as_f64().unwrap_or(0.0),
                el["lon"].as_f64().unwrap_or(0.0),
            )
        } else if let Some(c) = el.get("center") {
            (
                c["lat"].as_f64().unwrap_or(0.0),
                c["lon"].as_f64().unwrap_or(0.0),
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

/// รัน sync ครั้งเดียว (ดึง OSM → upsert DB)
pub async fn run_sync(db: &sea_orm::DatabaseConnection) {
    let tiles = region_tiles();
    info!("Gas station sync: เริ่ม {} tiles (ไทย/ลาว/มาเล/จีน)", tiles.len());

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Gas station sync: failed to create HTTP client: {}", e);
            return;
        }
    };

    let mut total_upserted = 0usize;
    let now = chrono::Utc::now();

    for (i, (south, west, north, east)) in tiles.iter().enumerate() {
        match fetch_osm(&client, *south, *west, *north, *east).await {
            Ok(stations) => {
                for g in &stations {
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
                        updated_at: Set(Some(now.into())),
                        ..Default::default()
                    };
                    let _ = gas_station::Entity::insert(am)
                        .on_conflict(
                            sea_query::OnConflict::columns([
                                gas_station::Column::Source,
                                gas_station::Column::ExternalId,
                            ])
                            .update_column(gas_station::Column::Lat)
                            .update_column(gas_station::Column::Lng)
                            .update_column(gas_station::Column::Name)
                            .update_column(gas_station::Column::Brand)
                            .update_column(gas_station::Column::UpdatedAt)
                            .to_owned(),
                        )
                        .exec(db)
                        .await;
                    total_upserted += 1;
                }
            }
            Err(e) => {
                warn!("Gas station sync: tile {}/{} failed ({},{},{},{}): {}", i + 1, tiles.len(), south, west, north, east, e);
            }
        }
        if i + 1 < tiles.len() {
            tokio::time::sleep(tokio::time::Duration::from_millis(DELAY_BETWEEN_TILES_MS)).await;
        }
    }

    info!("Gas station sync: เสร็จ {} จุด", total_upserted);
}

/// Spawn background task: รัน sync ครั้งแรกหลัง 30 วินาที แล้วรันวันละ 1 รอบ
pub fn spawn_scheduler(db: sea_orm::DatabaseConnection) {
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        run_sync(&db).await;

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            run_sync(&db).await;
        }
    });
}
