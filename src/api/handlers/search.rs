use super::to_error;
use crate::AppState;
use crate::api::responses::*;
use crate::domain::engine::SearchEngine;
use crate::domain::query::{parse_aggregations, parse_pagination, parse_query, parse_sort};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub async fn count(
    Path(index): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(query_json): Json<Value>,
) -> Result<Json<CountResponse>, (StatusCode, Json<ErrorResponse>)> {
    let index_data = state.store.get_index(&index).ok_or_else(|| {
        to_error(
            StatusCode::NOT_FOUND,
            "index_not_found_exception",
            &format!("no such index [{}]", index),
        )
    })?;
    let query = parse_query(&query_json);
    let count = index_data
        .documents
        .iter()
        .filter(|d| query.matches(d))
        .count();
    Ok(Json(CountResponse {
        count,
        _shards: ShardsInfo::default(),
    }))
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
    let agg_definitions = parse_aggregations(&query_json);

    let filtered_docs =
        SearchEngine::search(&index_data.documents, query.as_ref(), sort, from, size);
    let hits: Vec<SearchHit> = filtered_docs
        .iter()
        .map(|doc| SearchHit {
            _index: index.clone(),
            _id: doc["_id"].as_str().unwrap_or("unknown").to_string(),
            _score: 1.0,
            _source: doc.clone(),
        })
        .collect();

    let mut aggregations = None;
    if !agg_definitions.is_empty() {
        let all_filtered = index_data
            .documents
            .iter()
            .filter(|d| query.matches(d))
            .cloned()
            .collect::<Vec<Value>>();
        let agg_results = SearchEngine::aggregate(&all_filtered, &agg_definitions);
        let mut map = HashMap::new();
        for res in agg_results {
            map.insert(
                res.name,
                AggregationBuckets {
                    buckets: res
                        .buckets
                        .into_iter()
                        .map(|b| BucketResponse {
                            key: b.key,
                            doc_count: b.doc_count,
                        })
                        .collect(),
                },
            );
        }
        aggregations = Some(map);
    }

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
        aggregations,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::handlers::setup_state;
    use crate::domain::mapping::Mapping;

    #[tokio::test]
    async fn should_search_with_aggregations() {
        let state = setup_state();
        let index = "search-agg".to_string();
        state.store.create_index(index.clone(), Mapping::default());
        state
            .store
            .add_document(&index, json!({ "category": "A" }))
            .unwrap();
        state
            .store
            .add_document(&index, json!({ "category": "A" }))
            .unwrap();
        state
            .store
            .add_document(&index, json!({ "category": "B" }))
            .unwrap();

        let query = json!({ "aggs": { "cats": { "terms": { "field": "category" } } } });
        let Json(response) = search(Path(index), State(state), Json(query))
            .await
            .unwrap();

        let aggs = response.aggregations.as_ref().unwrap();
        let buckets = &aggs["cats"].buckets;
        assert_eq!(buckets.len(), 2);
    }

    #[tokio::test]
    async fn should_count_documents() {
        let state = setup_state();
        let index = "count-test".to_string();
        state.store.create_index(index.clone(), Mapping::default());
        state.store.add_document(&index, json!({ "v": 1 })).unwrap();
        state.store.add_document(&index, json!({ "v": 2 })).unwrap();

        let query = json!({ "query": { "term": { "v": 1 } } });
        let Json(response) = count(Path(index), State(state), Json(query)).await.unwrap();
        assert_eq!(response.count, 1);
    }
}