use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{reel, user};
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::AppState;

const MAX_VIDEO_SIZE: usize = 100 * 1024 * 1024; // 100MB

#[derive(Serialize)]
pub struct ReelResponse {
    pub id: String,
    pub video_url: String,
    pub caption: String,
    pub uploaded_by: String,
    pub created_at: String,
}

fn get_uploaded_by(u: &user::Model) -> String {
    u.display_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&u.email)
        .to_string()
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
) -> Result<Json<Vec<ReelResponse>>, AppError> {
    let reels = reel::Entity::find()
        .order_by_desc(reel::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let user_ids: Vec<Uuid> = reels
        .iter()
        .map(|r| r.user_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let users_map: std::collections::HashMap<Uuid, user::Model> = if user_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        user::Entity::find()
            .filter(user::Column::Id.is_in(user_ids))
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .into_iter()
            .map(|u| (u.id, u))
            .collect()
    };

    let out: Vec<ReelResponse> = reels
        .into_iter()
        .map(|r| {
            let uploaded_by = users_map
                .get(&r.user_id)
                .map(get_uploaded_by)
                .unwrap_or_else(|| "Unknown".to_string());
            ReelResponse {
                id: r.id.to_string(),
                video_url: format!("/api/reels/{}/video", r.id),
                caption: r.caption,
                uploaded_by,
                created_at: r.created_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(out))
}

pub async fn upload(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ReelResponse>, AppError> {
    let mut video_data: Option<Vec<u8>> = None;
    let mut caption = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart".into()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "video" {
            let data = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("Failed to read video".into()))?;
            if data.len() > MAX_VIDEO_SIZE {
                return Err(AppError::BadRequest("Video too large (max 100MB)".into()));
            }
            if data.is_empty() {
                return Err(AppError::BadRequest("Video file is empty".into()));
            }
            video_data = Some(data.to_vec());
        } else if name == "caption" {
            if let Ok(s) = field.text().await {
                caption = s.trim().to_string();
            }
        }
    }

    let video_data = video_data.ok_or_else(|| AppError::BadRequest("Missing video file".into()))?;

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let reels_dir = state.config.upload_dir.join("reels");
    fs::create_dir_all(&reels_dir)
        .await
        .map_err(|_| AppError::Internal)?;

    let ext = "mp4";
    let filename = format!("{}.{}", id, ext);
    let file_path = reels_dir.join(&filename);
    let mut file = fs::File::create(&file_path).await.map_err(|e| {
        tracing::error!("Failed to create file: {}", e);
        AppError::Internal
    })?;
    file.write_all(&video_data)
        .await
        .map_err(|_| AppError::Internal)?;
    file.sync_all().await.map_err(|_| AppError::Internal)?;
    drop(file);

    let _ = crate::media::compress_video(&file_path).await;

    let relative_path = format!("reels/{}", filename);

    let model = reel::ActiveModel {
        id: ActiveValue::Set(id),
        user_id: ActiveValue::Set(auth.id),
        caption: ActiveValue::Set(if caption.is_empty() {
            "วิดีโอของฉัน".to_string()
        } else {
            caption.clone()
        }),
        video_path: ActiveValue::Set(relative_path),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    model
        .insert(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let u = user::Entity::find_by_id(auth.id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .map(|u| get_uploaded_by(&u))
        .unwrap_or_else(|| auth.id.to_string());

    Ok(Json(ReelResponse {
        id: id.to_string(),
        video_url: format!("/api/reels/{}/video", id),
        caption: if caption.is_empty() {
            "วิดีโอของฉัน".to_string()
        } else {
            caption
        },
        uploaded_by: u,
        created_at: now.to_rfc3339(),
    }))
}

pub async fn serve_video(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError::NotFound("Reel not found".into()))?;

    let r = reel::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Reel not found".into()))?;

    let full_path = state.config.upload_dir.join(&r.video_path);
    let content = fs::read(&full_path).await.map_err(|e| {
        tracing::error!("Failed to read video file: {}", e);
        AppError::NotFound("Video file not found".into())
    })?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "video/mp4")],
        axum::body::Body::from(content),
    )
        .into_response())
}
