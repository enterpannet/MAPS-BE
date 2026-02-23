use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::user;
use crate::services::auth;
use axum::extract::State;
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    if existing.is_some() {
        return Err(AppError::BadRequest("อีเมลนี้ถูกใช้งานแล้ว".into()));
    }

    if req.password.len() < 6 {
        return Err(AppError::BadRequest(
            "รหัสผ่านต้องมีอย่างน้อย 6 ตัวอักษร".into(),
        ));
    }

    let password_hash = auth::hash_password(&req.password)?;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let u = user::ActiveModel {
        id: ActiveValue::Set(id),
        email: ActiveValue::Set(req.email.clone()),
        password_hash: ActiveValue::Set(password_hash),
        display_name: ActiveValue::Set(req.display_name.clone()),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    u.insert(&state.db).await.map_err(|_| AppError::Internal)?;

    let token = auth::create_token(id, &state.config.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: id.to_string(),
            email: req.email,
            display_name: req.display_name,
        },
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = user::Entity::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    if !auth::verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = auth::create_token(user.id, &state.config.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id.to_string(),
            email: user.email,
            display_name: user.display_name,
        },
    }))
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
}

pub async fn update_profile(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let u = user::Entity::find_by_id(auth.id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::NotFound("User not found".into()))?;

    let email = u.email.clone();
    let display_name = req.display_name.as_ref().map(|s| s.trim().to_string());
    let display_name = if display_name.as_deref() == Some("") {
        None
    } else {
        display_name
    };

    let mut u: user::ActiveModel = u.into();
    u.display_name = Set(display_name.clone());
    u.updated_at = Set(chrono::Utc::now().into());
    u.update(&state.db).await.map_err(|_| AppError::Internal)?;

    Ok(Json(UserResponse {
        id: auth.id.to_string(),
        email,
        display_name,
    }))
}
