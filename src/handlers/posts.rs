use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{post, post_comment, user};
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::AppState;

const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB

#[derive(Clone, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub content: String,
    pub author: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct PostResponse {
    pub id: String,
    pub content: String,
    pub image_url: Option<String>,
    pub author: String,
    pub created_at: String,
    pub comments: Vec<CommentResponse>,
}

fn get_author(u: &user::Model) -> String {
    u.display_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&u.email)
        .to_string()
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
) -> Result<Json<Vec<PostResponse>>, AppError> {
    let posts = post::Entity::find()
        .order_by_desc(post::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let user_ids: Vec<Uuid> = posts
        .iter()
        .map(|p| p.user_id)
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

    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();
    let all_comments = if post_ids.is_empty() {
        vec![]
    } else {
        post_comment::Entity::find()
            .filter(post_comment::Column::PostId.is_in(post_ids))
            .order_by_asc(post_comment::Column::CreatedAt)
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
    };

    let comment_user_ids: Vec<Uuid> = all_comments
        .iter()
        .map(|c| c.user_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let comment_users: std::collections::HashMap<Uuid, user::Model> = if comment_user_ids.is_empty()
    {
        std::collections::HashMap::new()
    } else {
        user::Entity::find()
            .filter(user::Column::Id.is_in(comment_user_ids))
            .all(&state.db)
            .await
            .map_err(|_| AppError::Internal)?
            .into_iter()
            .map(|u| (u.id, u))
            .collect()
    };

    let comments_by_post: std::collections::HashMap<Uuid, Vec<CommentResponse>> = all_comments
        .into_iter()
        .map(|c| {
            let author = comment_users
                .get(&c.user_id)
                .map(get_author)
                .unwrap_or_else(|| "Unknown".to_string());
            (
                c.post_id,
                CommentResponse {
                    id: c.id.to_string(),
                    content: c.content,
                    author,
                    created_at: c.created_at.to_rfc3339(),
                },
            )
        })
        .fold(
            std::collections::HashMap::new(),
            |mut acc, (post_id, comment)| {
                acc.entry(post_id).or_default().push(comment);
                acc
            },
        );

    let out: Vec<PostResponse> = posts
        .into_iter()
        .map(|p| {
            let author = users_map
                .get(&p.user_id)
                .map(get_author)
                .unwrap_or_else(|| "Unknown".to_string());
            let comments = comments_by_post.get(&p.id).cloned().unwrap_or_default();
            PostResponse {
                id: p.id.to_string(),
                content: p.content,
                image_url: p
                    .image_path
                    .as_ref()
                    .map(|_| format!("/api/posts/{}/image", p.id)),
                author,
                created_at: p.created_at.to_rfc3339(),
                comments,
            }
        })
        .collect();

    Ok(Json(out))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<PostResponse>, AppError> {
    let mut content = String::new();
    let mut image_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart".into()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "content" {
            if let Ok(s) = field.text().await {
                content = s.trim().to_string();
            }
        } else if name == "image" {
            let content_type = field.content_type().map(|c| c.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("Failed to read image".into()))?;
            if !data.is_empty() {
                if data.len() > MAX_IMAGE_SIZE {
                    return Err(AppError::BadRequest("Image too large (max 10MB)".into()));
                }
                if let Some(ct) = content_type {
                    if !ct.starts_with("image/") {
                        return Err(AppError::BadRequest("File must be an image".into()));
                    }
                }
                image_data = Some(data.to_vec());
            }
        }
    }

    if content.is_empty() {
        return Err(AppError::BadRequest("Content is required".into()));
    }

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let image_path = if let Some(data) = image_data {
        let posts_dir = state.config.upload_dir.join("posts");
        fs::create_dir_all(&posts_dir)
            .await
            .map_err(|_| AppError::Internal)?;

        let data_clone = data.clone();
        let compressed =
            tokio::task::spawn_blocking(move || crate::media::compress_image(&data_clone))
                .await
                .ok()
                .and_then(|r| r.ok())
                .unwrap_or(data);
        let ext = "jpg";
        let filename = format!("{}.{}", id, ext);
        let file_path = posts_dir.join(&filename);
        let mut file = fs::File::create(&file_path).await.map_err(|e| {
            tracing::error!("Failed to create file: {}", e);
            AppError::Internal
        })?;
        file.write_all(&compressed)
            .await
            .map_err(|_| AppError::Internal)?;
        file.sync_all().await.map_err(|_| AppError::Internal)?;

        Some(format!("posts/{}", filename))
    } else {
        None
    };

    let has_image = image_path.is_some();

    let model = post::ActiveModel {
        id: ActiveValue::Set(id),
        user_id: ActiveValue::Set(auth.id),
        content: ActiveValue::Set(content.clone()),
        image_path: ActiveValue::Set(image_path),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    model
        .insert(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let author = user::Entity::find_by_id(auth.id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .map(|u| get_author(&u))
        .unwrap_or_else(|| auth.id.to_string());

    Ok(Json(PostResponse {
        id: id.to_string(),
        content: content.clone(),
        image_url: if has_image {
            Some(format!("/api/posts/{}/image", id))
        } else {
            None
        },
        author,
        created_at: now.to_rfc3339(),
        comments: vec![],
    }))
}

pub async fn serve_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError::NotFound("Post not found".into()))?;

    let p = post::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    let path = p
        .image_path
        .as_ref()
        .ok_or_else(|| AppError::NotFound("Post has no image".into()))?;

    let full_path = state.config.upload_dir.join(path);
    let content = fs::read(&full_path).await.map_err(|e| {
        tracing::error!("Failed to read image: {}", e);
        AppError::NotFound("Image not found".into())
    })?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/jpeg")],
        axum::body::Body::from(content),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

pub async fn create_comment(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(post_id): Path<String>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<CommentResponse>, AppError> {
    let post_id =
        Uuid::parse_str(&post_id).map_err(|_| AppError::BadRequest("Invalid post_id".into()))?;

    let _post = post::Entity::find_by_id(post_id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    let content = req.content.trim();
    if content.is_empty() {
        return Err(AppError::BadRequest("Content is required".into()));
    }

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let model = post_comment::ActiveModel {
        id: ActiveValue::Set(id),
        post_id: ActiveValue::Set(post_id),
        user_id: ActiveValue::Set(auth.id),
        content: ActiveValue::Set(content.to_string()),
        created_at: ActiveValue::Set(now.into()),
        ..Default::default()
    };

    model
        .insert(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    let author = user::Entity::find_by_id(auth.id)
        .one(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .map(|u| get_author(&u))
        .unwrap_or_else(|| auth.id.to_string());

    Ok(Json(CommentResponse {
        id: id.to_string(),
        content: content.to_string(),
        author,
        created_at: now.to_rfc3339(),
    }))
}
