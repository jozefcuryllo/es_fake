use super::to_error;
use crate::AppState;
use crate::api::responses::{ErrorResponse, IndexResponse, ShardsInfo};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;

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
    Ok(Json(json!({ "_index": index, "_id": id, "_source": doc })))
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

pub async fn bulk(State(state): State<Arc<AppState>>, body: String) -> Json<Value> {
    let mut results = Vec::new();
    let mut lines = body.lines();

    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(action_json) = serde_json::from_str::<Value>(line) {
            let action_type = action_json
                .as_object()
                .and_then(|obj| obj.keys().next())
                .cloned();

            match action_type.as_deref() {
                Some("index") | Some("create") => {
                    let act = &action_json[action_type.as_ref().unwrap()];
                    let index_name = act["_index"].as_str().unwrap_or("unknown").to_string();
                    let id = act["_id"].as_str().map(|s| s.to_string());

                    if let Some(data_line) = lines.next() {
                        if let Ok(mut doc) = serde_json::from_str::<Value>(data_line) {
                            if let Some(doc_id) = id {
                                if let Some(obj) = doc.as_object_mut() {
                                    obj.insert("_id".to_string(), Value::String(doc_id.clone()));
                                }
                            }
                            let res = state.store.add_document(&index_name, doc);
                            results.push(json!({
                                "index": {
                                    "_index": index_name,
                                    "_id": res.as_ref().ok(),
                                    "status": if res.is_ok() { 201 } else { 400 },
                                    "result": if res.is_ok() { "created" } else { "error" }
                                }
                            }));
                        }
                    }
                }
                Some("update") => {
                    let act = &action_json["update"];
                    let index_name = act["_index"].as_str().unwrap_or("unknown").to_string();
                    let id = act["_id"].as_str().unwrap_or_default().to_string();

                    if let Some(data_line) = lines.next() {
                        if let Ok(body) = serde_json::from_str::<Value>(data_line) {
                            let patch = body.get("doc").cloned().unwrap_or(body);
                            let res = state.store.patch_document(&index_name, &id, patch);
                            results.push(json!({
                                "update": {
                                    "_index": index_name,
                                    "_id": id,
                                    "status": if res.is_ok() { 200 } else { 404 },
                                    "result": if res.is_ok() { "updated" } else { "error" }
                                }
                            }));
                        }
                    }
                }
                Some("delete") => {
                    let act = &action_json["delete"];
                    let index_name = act["_index"].as_str().unwrap_or("unknown").to_string();
                    let id = act["_id"].as_str().unwrap_or_default().to_string();
                    let deleted = state.store.delete_document(&index_name, &id);
                    results.push(json!({
                        "delete": {
                            "_index": index_name,
                            "_id": id,
                            "status": if deleted { 200 } else { 404 },
                            "result": if deleted { "deleted" } else { "not_found" }
                        }
                    }));
                }
                _ => {}
            }
        }
    }

    Json(json!({ "took": 1, "errors": false, "items": results }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::handlers::setup_state;
    use crate::domain::mapping::Mapping;

    #[tokio::test]
    async fn should_index_and_get_document() {
        let state = setup_state();
        let index = "docs".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let doc = json!({ "title": "test" });
        let res = index_document(Path(index.clone()), State(state.clone()), Json(doc))
            .await
            .unwrap();
        let id = res._id.clone();

        let fetched = get_document(Path((index, id)), State(state)).await.unwrap();
        assert_eq!(fetched["_source"]["title"], "test");
    }

    #[tokio::test]
    async fn should_handle_partial_update() {
        let state = setup_state();
        let index = "updates".to_string();
        state.store.create_index(index.clone(), Mapping::default());
        let id = state
            .store
            .add_document(&index, json!({ "a": 1, "b": 2 }))
            .unwrap();

        let update = json!({ "doc": { "b": 3 } });
        let _ = update_document(
            Path((index.clone(), id.clone())),
            State(state.clone()),
            Json(update),
        )
        .await
        .unwrap();

        let doc = state.store.get_document(&index, &id).unwrap();
        assert_eq!(doc["a"], 1);
        assert_eq!(doc["b"], 3);
    }

    #[tokio::test]
    async fn should_return_404_on_missing_document() {
        let state = setup_state();
        let result = get_document(Path(("none".into(), "1".into())), State(state)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn should_handle_full_bulk_workflow() {
        let state = setup_state();
        let index = "bulk-test".to_string();
        state.store.create_index(index.clone(), Mapping::default());

        let bulk_body = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            json!({"index": {"_index": &index, "_id": "1"}}),
            json!({"field": "v1"}),
            json!({"update": {"_index": &index, "_id": "1"}}),
            json!({"doc": {"field": "v2"}}),
            json!({"delete": {"_index": &index, "_id": "1"}})
        );

        let response = bulk(State(state.clone()), bulk_body).await;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["index"]["result"], "created");
        assert_eq!(items[1]["update"]["result"], "updated");
        assert_eq!(items[2]["delete"]["result"], "deleted");
    }
}
