use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

use crate::AppState;
use crate::api::responses::*;
use crate::domain::engine::SearchEngine;
use crate::domain::mapping::Mapping;
use crate::domain::query::{parse_pagination, parse_query, parse_sort};

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

pub async fn info() -> Json<InfoResponse> {
    Json(InfoResponse {
        name: "es_fake".to_string(),
        cluster_name: "docker-cluster".to_string(),
        version: VersionInfo {
            number: "8.10.0".to_string(),
            build_flavor: "default".to_string(),
        },
        tagline: "You Know, for Search".to_string(),
    })
}

pub async fn cluster_health() -> Json<ClusterHealthResponse> {
    Json(ClusterHealthResponse {
        cluster_name: "docker-cluster".to_string(),
        status: "green".to_string(),
        timed_out: false,
        number_of_nodes: 1,
        number_of_data_nodes: 1,
        active_primary_shards: 1,
        active_shards: 1,
        relocating_shards: 0,
        initializing_shards: 0,
        unassigned_shards: 0,
        delayed_unassigned_shards: 0,
        number_of_pending_tasks: 0,
        number_of_in_flight_fetch: 0,
        task_max_waiting_in_queue_millis: 0,
        active_shards_percent_as_number: 100.0,
    })
}

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
    let m = match mapping {
        Some(Json(inner)) => inner,
        None => Mapping::default(),
    };

    state.store.create_index(index.clone(), m);

    Json(json!({
        "acknowledged": true,
        "shards_acknowledged": true,
        "index": index
    }))
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

pub async fn index_document(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(doc): Json<Value>,
) -> Result<Json<IndexResponse>, (StatusCode, Json<ErrorResponse>)> {
    let id = state
        .store
        .add_document(&index, doc)
        .map_err(|e| to_error(StatusCode::BAD_REQUEST, "mapper_parsing_exception", &e))?;

    Ok(Json(IndexResponse {
        _index: index,
        _id: id,
        result: "created".to_string(),
        _version: 1,
        _shards: ShardsInfo::default(),
    }))
}

pub async fn index_document_with_id(
    Path((index, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(mut doc): Json<Value>,
) -> Result<Json<IndexResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("_id".to_string(), Value::String(id.clone()));
    }

    let saved_id = state
        .store
        .add_document(&index, doc)
        .map_err(|e| to_error(StatusCode::BAD_REQUEST, "mapper_parsing_exception", &e))?;

    Ok(Json(IndexResponse {
        _index: index,
        _id: saved_id,
        result: "updated".to_string(),
        _version: 1,
        _shards: ShardsInfo::default(),
    }))
}

pub async fn update_document(
    Path((index, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<IndexResponse>, (StatusCode, Json<ErrorResponse>)> {
    let patch = body.get("doc").cloned().ok_or_else(|| {
        to_error(
            StatusCode::BAD_REQUEST,
            "action_request_validation_exception",
            "Validation Failed: 1: script or doc is missing;",
        )
    })?;

    let saved_id = state
        .store
        .patch_document(&index, &id, patch)
        .map_err(|e| {
            let status = if e.contains("index_not_found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            to_error(status, &e, &e)
        })?;

    Ok(Json(IndexResponse {
        _index: index,
        _id: saved_id,
        result: "updated".to_string(),
        _version: 1,
        _shards: ShardsInfo::default(),
    }))
}

pub async fn get_document(
    Path((index, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let doc = state.store.get_document(&index, &id).ok_or_else(|| {
        to_error(
            StatusCode::NOT_FOUND,
            "index_not_found_exception",
            "no such index or document",
        )
    })?;

    Ok(Json(json!({
        "_index": index,
        "_id": id,
        "_source": doc
    })))
}

pub async fn delete_document(
    Path((index, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.store.delete_document(&index, &id) {
        StatusCode::OK.into_response()
    } else {
        to_error(
            StatusCode::NOT_FOUND,
            "document_missing_exception",
            "document not found",
        )
        .into_response()
    }
}

pub async fn search(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(query_json): Json<Value>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();

    let index_data = state.store.get_index(&index).ok_or_else(|| {
        to_error(
            StatusCode::NOT_FOUND,
            "index_not_found_exception",
            &format!("no such index [{}]", index),
        )
    })?;

    let query = parse_query(&query_json);
    let sort = parse_sort(&query_json);
    let (from, size) = parse_pagination(&query_json);

    let filtered_docs =
        SearchEngine::search(&index_data.documents, query.as_ref(), sort, from, size);

    let hits: Vec<SearchHit> = filtered_docs
        .into_iter()
        .map(|doc| {
            let id = doc["_id"].as_str().unwrap_or("unknown").to_string();
            SearchHit {
                _index: index.clone(),
                _id: id,
                _score: 1.0,
                _source: doc,
            }
        })
        .collect();

    Ok(Json(SearchResponse {
        took: start.elapsed().as_millis(),
        timed_out: false,
        _shards: ShardsInfo::default(),
        hits: HitsMetadata {
            total: TotalHits {
                value: index_data.documents.len(),
                relation: "eq".to_string(),
            },
            max_score: if hits.is_empty() { None } else { Some(1.0) },
            hits,
        },
    }))
}

pub async fn bulk(State(state): State<Arc<AppState>>, body: String) -> Json<Value> {
    let mut results = Vec::new();
    let mut lines = body.lines();

    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(action_json) = serde_json::from_str::<Value>(line) {
            if let Some(index_action) = action_json.get("index") {
                let index_name = index_action["_index"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                if let Some(data_line) = lines.next() {
                    if let Ok(doc) = serde_json::from_str::<Value>(data_line) {
                        let res = state.store.add_document(&index_name, doc);
                        results.push(json!({
                            "index": {
                                "_index": index_name,
                                "status": if res.is_ok() { 201 } else { 400 },
                                "result": if res.is_ok() { "created" } else { "error" }
                            }
                        }));
                    }
                }
            }
        }
    }

    Json(json!({ "took": 1, "errors": false, "items": results }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::store::InMemoryStore;
    use axum::extract::Path;
    use serde_json::json;

    fn setup_state() -> Arc<AppState> {
        Arc::new(AppState {
            store: InMemoryStore::new(),
            auth_user: "elastic".to_string(),
            auth_password: "".to_string(),
            auth_enabled: false,
        })
    }

    #[tokio::test]
    async fn should_handle_delete_index_request() {
        let state = setup_state();
        state
            .store
            .create_index("delete-me".to_string(), Mapping::default());

        let response = delete_index(Path("delete-me".to_string()), State(state.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(state.store.get_index("delete-me").is_none());
    }

    #[tokio::test]
    async fn should_return_green_cluster_health() {
        let response = cluster_health().await;
        assert_eq!(response.status, "green");
    }

    #[tokio::test]
    async fn should_handle_refresh_request() {
        let state = setup_state();
        state
            .store
            .create_index("ref".to_string(), Mapping::default());

        let response = refresh(Path("ref".to_string()), State(state))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn should_handle_partial_update() {
        let state = setup_state();
        let index = "test".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        state
            .store
            .add_document(
                &index,
                json!({"_id": "1", "field1": "val1", "field2": "val2"}),
            )
            .unwrap();

        let update_body = json!({ "doc": { "field2": "updated" } });
        let _ = update_document(
            Path((index.clone(), "1".to_string())),
            State(state.clone()),
            Json(update_body),
        )
        .await
        .unwrap();

        let doc = state.store.get_document(&index, "1").unwrap();
        assert_eq!(doc["field1"], "val1");
        assert_eq!(doc["field2"], "updated");
    }

    #[tokio::test]
    async fn should_return_structured_error_on_missing_index() {
        let state = setup_state();
        let result = search(Path("missing".to_string()), State(state), Json(json!({}))).await;

        match result {
            Err((status, body)) => {
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(body.error.r#type, "index_not_found_exception");
                assert!(body.error.root_cause.len() > 0);
            }
            _ => panic!("Should fail"),
        }
    }
}
