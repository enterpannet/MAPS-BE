use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{fuel_record, room, room_member, trip};
use axum::extract::{Path, Query, State};
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreateFuelRecordRequest {
    pub trip_id: Option<String>,
    pub input_mode: String,
    pub distance_km: Option<f64>,
    pub fuel_liters: Option<f64>,
    pub km_per_liter: Option<f64>,
    pub price_per_liter: Option<f64>,
    pub total_cost: f64,
    pub receipt_image: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct FuelRecordResponse {
    pub id: String,
    pub room_id: Option<String>,
    pub room_name: Option<String>,
    pub trip_id: String,
    pub trip_name: Option<String>,
    pub user_id: String,
    pub input_mode: String,
    pub distance_km: Option<f64>,
    pub fuel_liters: Option<f64>,
    pub km_per_liter: Option<f64>,
    pub price_per_liter: Option<f64>,
    pub total_cost: f64,
    pub receipt_image: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(room_id): Path<String>,
    Json(req): Json<CreateFuelRecordRequest>,
) -> Result<Json<FuelRecordResponse>, AppError> {
    let room_id =
        Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;

    let _member = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;

    let trip_id = if let Some(tid) = &req.trip_id {
        let tid =
            Uuid::parse_str(tid).map_err(|_| AppError::BadRequest("Invalid trip_id".into()))?;
        let t = trip::Entity::find_by_id(tid)
            .one(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .ok_or_else(|| AppError::NotFound("Trip not found".into()))?;
        if t.room_id != room_id {
            return Err(AppError::BadRequest(
                "Trip does not belong to this room".into(),
            ));
        }
        tid
    } else {
        let existing = trip::Entity::find()
            .filter(trip::Column::RoomId.eq(room_id))
            .order_by_asc(trip::Column::CreatedAt)
            .one(&state.db)
            .await
            .map_err(|_| AppError::Internal)?;
        match existing {
            Some(t) => t.id,
            None => {
                let new_id = Uuid::new_v4();
                let now = chrono::Utc::now();
                let default_trip = trip::ActiveModel {
                    id: ActiveValue::Set(new_id),
                    room_id: ActiveValue::Set(room_id),
                    name: ActiveValue::Set("ทริปหลัก".to_string()),
                    created_at: ActiveValue::Set(now.into()),
                    ..Default::default()
                };
                default_trip
                    .insert(&state.db)
                    .await
                    .map_err(|_| AppError::Internal)?;
                new_id
            }
        }
    };

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let input_mode = req.input_mode.clone();
    let receipt_image = req.receipt_image.clone();
    let note = req.note.clone();

    let record = fuel_record::ActiveModel {
        id: ActiveValue::Set(id),
        room_id: ActiveValue::Set(room_id),
        trip_id: ActiveValue::Set(trip_id),
        user_id: ActiveValue::Set(auth.id),
        input_mode: ActiveValue::Set(input_mode.clone()),
        distance_km: ActiveValue::Set(req.distance_km),
        fuel_liters: ActiveValue::Set(req.fuel_liters),
        km_per_liter: ActiveValue::Set(req.km_per_liter),
        price_per_liter: ActiveValue::Set(req.price_per_liter),
        total_cost: ActiveValue::Set(req.total_cost),
        receipt_image: ActiveValue::Set(receipt_image.clone()),
        note: ActiveValue::Set(note.clone()),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    record
        .insert(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let trip_model = trip::Entity::find_by_id(trip_id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .unwrap();
    let room_model = room::Entity::find_by_id(room_id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .unwrap();

    Ok(Json(FuelRecordResponse {
        id: id.to_string(),
        room_id: Some(room_id.to_string()),
        room_name: Some(room_model.name),
        trip_id: trip_id.to_string(),
        trip_name: Some(trip_model.name),
        user_id: auth.id.to_string(),
        input_mode,
        distance_km: req.distance_km,
        fuel_liters: req.fuel_liters,
        km_per_liter: req.km_per_liter,
        price_per_liter: req.price_per_liter,
        total_cost: req.total_cost,
        receipt_image,
        note,
        created_at: now.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
pub struct ListFuelQuery {
    pub trip_id: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(room_id): Path<String>,
    Query(query): Query<ListFuelQuery>,
) -> Result<Json<Vec<FuelRecordResponse>>, AppError> {
    let room_id =
        Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;

    let _member = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;

    let mut q = fuel_record::Entity::find().filter(fuel_record::Column::RoomId.eq(room_id));
    if let Some(tid) = &query.trip_id {
        if let Ok(tid_uuid) = Uuid::parse_str(tid) {
            q = q.filter(fuel_record::Column::TripId.eq(tid_uuid));
        }
    }
    let records = q
        .order_by_desc(fuel_record::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let room_ids: Vec<Uuid> = records
        .iter()
        .map(|r| r.room_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let trip_ids: Vec<Uuid> = records
        .iter()
        .map(|r| r.trip_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let rooms_map: std::collections::HashMap<Uuid, String> = room::Entity::find()
        .filter(room::Column::Id.is_in(room_ids))
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .into_iter()
        .map(|r| (r.id, r.name))
        .collect();
    let trips_map: std::collections::HashMap<Uuid, String> = trip::Entity::find()
        .filter(trip::Column::Id.is_in(trip_ids))
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .into_iter()
        .map(|t| (t.id, t.name))
        .collect();

    let out: Vec<FuelRecordResponse> = records
        .into_iter()
        .map(|r| FuelRecordResponse {
            id: r.id.to_string(),
            room_id: Some(r.room_id.to_string()),
            room_name: rooms_map.get(&r.room_id).cloned(),
            trip_id: r.trip_id.to_string(),
            trip_name: trips_map.get(&r.trip_id).cloned(),
            user_id: r.user_id.to_string(),
            input_mode: r.input_mode,
            distance_km: r.distance_km,
            fuel_liters: r.fuel_liters,
            km_per_liter: r.km_per_liter,
            price_per_liter: r.price_per_liter,
            total_cost: r.total_cost,
            receipt_image: r.receipt_image,
            note: r.note,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(out))
}

pub async fn list_all(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
) -> Result<Json<Vec<FuelRecordResponse>>, AppError> {
    let memberships = room_member::Entity::find()
        .filter(room_member::Column::UserId.eq(auth.id))
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;
    let room_ids: Vec<Uuid> = memberships.into_iter().map(|m| m.room_id).collect();
    if room_ids.is_empty() {
        return Ok(Json(vec![]));
    }

    let records = fuel_record::Entity::find()
        .filter(fuel_record::Column::RoomId.is_in(room_ids))
        .order_by_desc(fuel_record::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let room_ids: Vec<Uuid> = records
        .iter()
        .map(|r| r.room_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let trip_ids: Vec<Uuid> = records
        .iter()
        .map(|r| r.trip_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let rooms_map: std::collections::HashMap<Uuid, String> = if room_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        room::Entity::find()
            .filter(room::Column::Id.is_in(room_ids))
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .into_iter()
            .map(|r| (r.id, r.name))
            .collect()
    };
    let trips_map: std::collections::HashMap<Uuid, String> = if trip_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        trip::Entity::find()
            .filter(trip::Column::Id.is_in(trip_ids))
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .into_iter()
            .map(|t| (t.id, t.name))
            .collect()
    };

    let out: Vec<FuelRecordResponse> = records
        .into_iter()
        .map(|r| FuelRecordResponse {
            id: r.id.to_string(),
            room_id: Some(r.room_id.to_string()),
            room_name: rooms_map.get(&r.room_id).cloned(),
            trip_id: r.trip_id.to_string(),
            trip_name: trips_map.get(&r.trip_id).cloned(),
            user_id: r.user_id.to_string(),
            input_mode: r.input_mode,
            distance_km: r.distance_km,
            fuel_liters: r.fuel_liters,
            km_per_liter: r.km_per_liter,
            price_per_liter: r.price_per_liter,
            total_cost: r.total_cost,
            receipt_image: r.receipt_image,
            note: r.note,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(out))
}
