pub mod cluster;
pub mod documents;
pub mod indices;
pub mod search;

use axum::Json;
use axum::http::StatusCode;
use crate::api::responses::{ErrorResponse, create_error_response};

fn to_error(
    status: StatusCode,
    error_type: &str,
    reason: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(create_error_response(status.as_u16(), error_type, reason)),
    )
}

#[cfg(test)]
fn setup_state() -> std::sync::Arc<crate::AppState> {
    std::sync::Arc::new(crate::AppState {
        store: crate::repository::store::InMemoryStore::new(),
        auth_user: "elastic".to_string(),
        auth_password: "".to_string(),
        auth_enabled: false,
    })
}