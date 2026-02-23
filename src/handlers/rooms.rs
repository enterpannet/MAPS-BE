use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{room, room_member};
use crate::services::auth;
use axum::extract::State;
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct JoinRoomRequest {
    pub code: String,
}

#[derive(Serialize)]
pub struct RoomResponse {
    pub id: String,
    pub name: String,
    pub code: String,
    pub owner_id: String,
}

#[derive(Serialize)]
pub struct RoomWithJoined {
    pub id: String,
    pub name: String,
    pub code: String,
    pub owner_id: String,
    pub joined_at: String,
}

pub async fn list_my_rooms(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
) -> Result<Json<Vec<RoomWithJoined>>, AppError> {
    let memberships = room_member::Entity::find()
        .filter(room_member::Column::UserId.eq(auth.id))
        .order_by_desc(room_member::Column::JoinedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let mut result = Vec::new();
    for m in memberships {
        let room = room::Entity::find_by_id(m.room_id)
            .one(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .ok_or_else(|| AppError::NotFound("Room not found".into()))?;

        result.push(RoomWithJoined {
            id: room.id.to_string(),
            name: room.name.clone(),
            code: room.code.clone(),
            owner_id: room.owner_id.to_string(),
            joined_at: m.joined_at.to_rfc3339(),
        });
    }

    Ok(Json(result))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<CreateRoomRequest>,
) -> Result<Json<RoomResponse>, AppError> {
    let id = Uuid::new_v4();
    let code = auth::generate_room_code();
    let now = chrono::Utc::now();
    let name = req.name.clone();

    let r = room::ActiveModel {
        id: ActiveValue::Set(id),
        name: ActiveValue::Set(name.clone()),
        code: ActiveValue::Set(code.clone()),
        owner_id: ActiveValue::Set(auth.id),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    r.insert(&state.db).await.map_err(|_| AppError::Internal)?;

    // Owner joins room
    let rm = room_member::ActiveModel {
        room_id: ActiveValue::Set(id),
        user_id: ActiveValue::Set(auth.id),
        joined_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };
    rm.insert(&state.db).await.map_err(|_| AppError::Internal)?;

    Ok(Json(RoomResponse {
        id: id.to_string(),
        name,
        code,
        owner_id: auth.id.to_string(),
    }))
}

pub async fn join_by_code(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<JoinRoomRequest>,
) -> Result<Json<RoomResponse>, AppError> {
    let room = room::Entity::find()
        .filter(room::Column::Code.eq(req.code.trim()))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Room not found".into()))?;

    let existing = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room.id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    if existing.is_none() {
        let rm = room_member::ActiveModel {
            room_id: ActiveValue::Set(room.id),
            user_id: ActiveValue::Set(auth.id),
            joined_at: ActiveValue::Set(chrono::Utc::now().into()),
            ..Default::default()
        };
        rm.insert(&state.db).await.map_err(|_| AppError::Internal)?;
    }

    Ok(Json(RoomResponse {
        id: room.id.to_string(),
        name: room.name.clone(),
        code: room.code.clone(),
        owner_id: room.owner_id.to_string(),
    }))
}
