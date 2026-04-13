use axum::{
    Json,
    extract::Request,
    extract::State,
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::services::jwt::JwtService;

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

pub async fn auth_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let Some(auth) = auth_header else {
        return unauthorized_response("Missing bearer token");
    };
    if !auth.starts_with("Bearer ") {
        return unauthorized_response("Invalid authorization scheme");
    }

    let token = &auth[7..];
    let claims = match jwt_service.validate_access_token(token) {
        Ok(claims) => claims,
        Err(_) => return unauthorized_response("Invalid or expired token"),
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(value) => value,
        Err(_) => return unauthorized_response("Malformed token subject"),
    };
    let session_id = match Uuid::parse_str(&claims.sid) {
        Ok(value) => value,
        Err(_) => return unauthorized_response("Malformed token session"),
    };

    req.extensions_mut().insert(AuthenticatedUser {
        user_id,
        session_id,
    });

    next.run(req).await
}

fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}
