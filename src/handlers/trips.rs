use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{room_member, trip};
use axum::extract::{Path, State};
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreateTripRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct TripResponse {
    pub id: String,
    pub room_id: String,
    pub name: String,
    pub created_at: String,
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<Vec<TripResponse>>, AppError> {
    let room_id = Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;

    let _member = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;

    let trips = trip::Entity::find()
        .filter(trip::Column::RoomId.eq(room_id))
        .order_by_asc(trip::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let out: Vec<TripResponse> = trips
        .into_iter()
        .map(|t| TripResponse {
            id: t.id.to_string(),
            room_id: t.room_id.to_string(),
            name: t.name,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(out))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(room_id): Path<String>,
    Json(req): Json<CreateTripRequest>,
) -> Result<Json<TripResponse>, AppError> {
    let room_id = Uuid::parse_str(&room_id).map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;

    let _member = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;

    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("ชื่อทริปไม่สามารถว่างได้".into()));
    }

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let record = trip::ActiveModel {
        id: ActiveValue::Set(id),
        room_id: ActiveValue::Set(room_id),
        name: ActiveValue::Set(name.to_string()),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    record.insert(&state.db).await.map_err(|_| AppError::Internal)?;

    Ok(Json(TripResponse {
        id: id.to_string(),
        room_id: room_id.to_string(),
        name: name.to_string(),
        created_at: now.to_rfc3339(),
    }))
}
