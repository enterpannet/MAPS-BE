use crate::error::AppError;
use crate::services::auth;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};
use uuid::Uuid;

pub struct AuthUser(pub auth::AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        let token = auth_header.ok_or((StatusCode::UNAUTHORIZED, "Missing token"))?;

        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".into());
        let claims = auth::decode_token(token, &secret)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token"))?;

        let id = Uuid::parse_str(&claims.sub)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token"))?;

        Ok(AuthUser(auth::AuthUser {
            id,
            email: String::new(),
            display_name: None,
        }))
    }
}
