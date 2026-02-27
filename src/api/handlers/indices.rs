use super::to_error;
use crate::AppState;
use crate::api::responses::{RefreshResponse, ShardsInfo};
use crate::domain::mapping::Mapping;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn check_index(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    if state.store.get_index(&index).is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

pub async fn create_index(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
    mapping: Option<Json<Mapping>>,
) -> Json<Value> {
    let m = mapping.map(|Json(inner)| inner).unwrap_or_default();
    state.store.create_index(index.clone(), m);
    Json(json!({
        "acknowledged": true,
        "shards_acknowledged": true,
        "index": index
    }))
}

pub async fn get_mapping(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.store.get_index(&index) {
        Some(idx) => Json(json!({ &index: { "mappings": idx.mapping } })).into_response(),
        None => to_error(
            StatusCode::NOT_FOUND,
            "index_not_found_exception",
            &format!("no such index [{}]", index),
        )
        .into_response(),
    }
}

pub async fn put_mapping(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mapping): Json<Mapping>,
) -> impl IntoResponse {
    match state.store.update_mapping(&index, mapping) {
        Ok(_) => Json(json!({ "acknowledged": true })).into_response(),
        Err(e) => to_error(StatusCode::NOT_FOUND, "index_not_found_exception", &e).into_response(),
    }
}

pub async fn get_settings(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.store.get_index(&index).is_some() {
        Json(json!({
            &index: {
                "settings": {
                    "index": {
                        "number_of_shards": "1",
                        "number_of_replicas": "0",
                        "provided_name": index
                    }
                }
            }
        }))
        .into_response()
    } else {
        to_error(
            StatusCode::NOT_FOUND,
            "index_not_found_exception",
            &format!("no such index [{}]", index),
        )
        .into_response()
    }
}

pub async fn delete_index(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.store.delete_index(&index) {
        Json(json!({ "acknowledged": true })).into_response()
    } else {
        to_error(
            StatusCode::NOT_FOUND,
            "index_not_found_exception",
            &format!("no such index [{}]", index),
        )
        .into_response()
    }
}

pub async fn refresh(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.store.refresh(&index) {
        Ok(_) => Json(RefreshResponse {
            _shards: ShardsInfo::default(),
        })
        .into_response(),
        Err(e) => to_error(StatusCode::NOT_FOUND, &e, &e).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::handlers::setup_state;

    #[tokio::test]
    async fn should_create_and_check_index() {
        let state = setup_state();
        let index = "test-index".to_string();

        let _ = create_index(Path(index.clone()), State(state.clone()), None).await;
        let status = check_index(Path(index), State(state)).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn should_get_index_settings() {
        let state = setup_state();
        let index = "settings-test".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let response = get_settings(Path(index.clone()), State(state))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn should_handle_mapping_lifecycle() {
        let state = setup_state();
        let index = "mapping-life".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let new_mapping = json!({ "properties": { "field": { "type": "text" } } });
        let m: Mapping = serde_json::from_value(new_mapping).unwrap();
        put_mapping(Path(index.clone()), State(state.clone()), Json(m)).await;

        let response = get_mapping(Path(index), State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn should_handle_delete_index() {
        let state = setup_state();
        let index = "to-delete".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let response = delete_index(Path(index.clone()), State(state.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(state.store.get_index(&index).is_none());
    }

    #[tokio::test]
    async fn should_handle_head_index() {
        let state = setup_state();
        let index = "head-test".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let status = check_index(Path(index), State(state)).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn should_return_404_for_non_existent_index_head() {
        let state = setup_state();
        let status = check_index(Path("ghost".to_string()), State(state)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn should_handle_refresh_as_noop_success() {
        let state = setup_state();
        let index = "refresh-test".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let response = refresh(Path(index), State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
