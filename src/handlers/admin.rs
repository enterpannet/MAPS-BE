use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::user;
use crate::AppState;
use axum::extract::{Path, State};
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
    pub created_at: String,
}

fn to_response(u: user::Model) -> UserResponse {
    UserResponse {
        id: u.id.to_string(),
        email: u.email,
        display_name: u.display_name,
        role: u.role,
        created_at: u.created_at.to_rfc3339(),
    }
}

fn require_admin(auth: &crate::services::auth::AuthUser) -> Result<(), AppError> {
    if auth.role != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// GET /api/admin/users — list all users (admin only)
pub async fn list_users(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    require_admin(&auth)?;

    let users = user::Entity::find()
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(users.into_iter().map(to_response).collect()))
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

/// PATCH /api/admin/users/:id/role — change a user's role (admin only)
pub async fn update_role_handler(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(user_id): Path<String>,
    Json(body): Json<UpdateRoleRequest>,
) -> Result<Json<UserResponse>, AppError> {
    require_admin(&auth)?;

    if body.role != "admin" && body.role != "member" {
        return Err(AppError::BadRequest("role must be 'admin' or 'member'".into()));
    }

    let id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user id".into()))?;

    // Prevent admin from demoting themselves
    if id == auth.id && body.role != "admin" {
        return Err(AppError::BadRequest(
            "Cannot change your own role".into(),
        ));
    }

    let user = user::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let mut active: user::ActiveModel = user.into();
    active.role = ActiveValue::Set(body.role);

    let updated = active
        .update(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(to_response(updated)))
}
