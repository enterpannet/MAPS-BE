use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{room_member, waypoint};
use axum::extract::{Path, State};
use axum::Json;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

async fn check_room_member(
    db: &sea_orm::DatabaseConnection,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let _ = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;
    Ok(())
}

#[derive(Deserialize)]
pub struct CreateWaypointRequest {
    pub name: String,
    pub waypoint_type: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Serialize)]
pub struct WaypointResponse {
    pub id: String,
    pub room_id: String,
    pub name: String,
    pub waypoint_type: String,
    pub lat: f64,
    pub lng: f64,
    pub sort_order: i32,
    pub created_at: String,
}

fn valid_type(t: &str) -> bool {
    matches!(t, "destination" | "rest" | "stopover")
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<Vec<WaypointResponse>>, AppError> {
    let room_id =
        Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;
    check_room_member(&state.db, room_id, auth.id).await?;

    let waypoints = waypoint::Entity::find()
        .filter(waypoint::Column::RoomId.eq(room_id))
        .order_by_asc(waypoint::Column::SortOrder)
        .order_by_asc(waypoint::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let out: Vec<WaypointResponse> = waypoints
        .into_iter()
        .map(|w| WaypointResponse {
            id: w.id.to_string(),
            room_id: w.room_id.to_string(),
            name: w.name.clone(),
            waypoint_type: w.waypoint_type.clone(),
            lat: w.lat,
            lng: w.lng,
            sort_order: w.sort_order,
            created_at: w.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(out))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(room_id): Path<String>,
    Json(req): Json<CreateWaypointRequest>,
) -> Result<Json<WaypointResponse>, AppError> {
    let room_id =
        Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;
    check_room_member(&state.db, room_id, auth.id).await?;

    if !valid_type(&req.waypoint_type) {
        return Err(AppError::BadRequest(
            "waypoint_type must be destination, rest, or stopover".into(),
        ));
    }

    let count = waypoint::Entity::find()
        .filter(waypoint::Column::RoomId.eq(room_id))
        .count(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let w = waypoint::ActiveModel {
        id: ActiveValue::Set(id),
        room_id: ActiveValue::Set(room_id),
        name: ActiveValue::Set(req.name.trim().to_string()),
        waypoint_type: ActiveValue::Set(req.waypoint_type),
        lat: ActiveValue::Set(req.lat),
        lng: ActiveValue::Set(req.lng),
        sort_order: ActiveValue::Set(count as i32),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    let w = w.insert(&state.db).await.map_err(|_| AppError::Internal)?;

    Ok(Json(WaypointResponse {
        id: w.id.to_string(),
        room_id: w.room_id.to_string(),
        name: w.name.clone(),
        waypoint_type: w.waypoint_type.clone(),
        lat: w.lat,
        lng: w.lng,
        sort_order: w.sort_order,
        created_at: w.created_at.to_rfc3339(),
    }))
}

pub async fn delete(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path((room_id, waypoint_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let room_id =
        Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;
    let waypoint_id = Uuid::parse_str(&waypoint_id)
        .map_err(|_| AppError::BadRequest("Invalid waypoint_id".into()))?;

    check_room_member(&state.db, room_id, auth.id).await?;

    let w = waypoint::Entity::find_by_id(waypoint_id)
        .filter(waypoint::Column::RoomId.eq(room_id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Waypoint not found".into()))?;

    w.delete(&state.db).await.map_err(|_| AppError::Internal)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
