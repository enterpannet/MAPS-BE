use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{location, room_member, user};
use axum::extract::State;
use axum::Json;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct ReportLocationRequest {
    pub room_id: String,
    pub lat: f64,
    pub lng: f64,
    pub accuracy: Option<f32>,
}

#[derive(Serialize)]
pub struct LocationResponse {
    pub id: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub lat: f64,
    pub lng: f64,
    pub accuracy: Option<f32>,
    pub created_at: String,
}

pub async fn report(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<ReportLocationRequest>,
) -> Result<Json<LocationResponse>, AppError> {
    let room_id = Uuid::parse_str(&req.room_id)
        .map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;

    // Verify user is in room
    let _member = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let loc = location::ActiveModel {
        id: ActiveValue::Set(id),
        room_id: ActiveValue::Set(room_id),
        user_id: ActiveValue::Set(auth.id),
        lat: ActiveValue::Set(req.lat),
        lng: ActiveValue::Set(req.lng),
        accuracy: ActiveValue::Set(req.accuracy),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    loc.insert(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let display_name = user::Entity::find_by_id(auth.id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .and_then(|u| u.display_name);

    // Broadcast to WebSocket room (create channel if first location in room)
    let msg = serde_json::json!({
        "id": id.to_string(),
        "user_id": auth.id.to_string(),
        "display_name": display_name,
        "lat": req.lat,
        "lng": req.lng,
        "accuracy": req.accuracy,
        "created_at": now.to_rfc3339(),
    });
    let mut rooms = state.rooms.write().await;
    let tx = rooms
        .entry(room_id)
        .or_insert_with(|| tokio::sync::broadcast::channel(100).0);
    let _ = tx.send(msg.to_string());

    Ok(Json(LocationResponse {
        id: id.to_string(),
        user_id: auth.id.to_string(),
        display_name,
        lat: req.lat,
        lng: req.lng,
        accuracy: req.accuracy,
        created_at: now.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
pub struct ListParams {
    pub room_id: String,
    pub limit: Option<u64>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    axum::extract::Query(params): axum::extract::Query<ListParams>,
) -> Result<Json<Vec<LocationResponse>>, AppError> {
    let room_id = Uuid::parse_str(&params.room_id)
        .map_err(|_| AppError::BadRequest("Invalid room_id".into()))?;

    let _member = room_member::Entity::find()
        .filter(room_member::Column::RoomId.eq(room_id))
        .filter(room_member::Column::UserId.eq(auth.id))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::Forbidden)?;

    let limit = params.limit.unwrap_or(100).min(500);

    let locations = location::Entity::find()
        .filter(location::Column::RoomId.eq(room_id))
        .order_by_desc(location::Column::CreatedAt)
        .limit(limit)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let user_ids: Vec<Uuid> = locations
        .iter()
        .map(|l| l.user_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let users_map: std::collections::HashMap<Uuid, Option<String>> = if user_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        user::Entity::find()
            .filter(user::Column::Id.is_in(user_ids))
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .into_iter()
            .map(|u| (u.id, u.display_name))
            .collect()
    };

    let out: Vec<LocationResponse> = locations
        .into_iter()
        .map(|l| LocationResponse {
            id: l.id.to_string(),
            user_id: l.user_id.to_string(),
            display_name: users_map.get(&l.user_id).and_then(|d| d.clone()),
            lat: l.lat,
            lng: l.lng,
            accuracy: l.accuracy,
            created_at: l.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(out))
}
