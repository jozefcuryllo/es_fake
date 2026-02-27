use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct InfoResponse {
    pub name: String,
    pub cluster_name: String,
    pub version: VersionInfo,
    pub tagline: String,
}

#[derive(Serialize, Clone)]
pub struct VersionInfo {
    pub number: String,
    pub build_flavor: String,
}

#[derive(Serialize, Clone)]
pub struct IndexResponse {
    pub _index: String,
    pub _id: String,
    pub result: String,
    pub _version: u32,
    pub _shards: ShardsInfo,
}

#[derive(Serialize, Clone)]
pub struct ShardsInfo {
    pub total: u32,
    pub successful: u32,
    pub failed: u32,
    pub skipped: Option<u32>,
}

impl Default for ShardsInfo {
    fn default() -> Self {
        Self {
            total: 1,
            successful: 1,
            failed: 0,
            skipped: Some(0),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct SearchResponse {
    pub took: u128,
    pub timed_out: bool,
    pub _shards: ShardsInfo,
    pub hits: HitsMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<HashMap<String, AggregationBuckets>>,
}

#[derive(Serialize, Clone)]
pub struct AggregationBuckets {
    pub buckets: Vec<BucketResponse>,
}

#[derive(Serialize, Clone)]
pub struct BucketResponse {
    pub key: Value,
    pub doc_count: usize,
}

#[derive(Serialize, Clone)]
pub struct HitsMetadata {
    pub total: TotalHits,
    pub max_score: Option<f64>,
    pub hits: Vec<SearchHit>,
}

#[derive(Serialize, Clone)]
pub struct TotalHits {
    pub value: usize,
    pub relation: String,
}

#[derive(Serialize, Clone)]
pub struct SearchHit {
    pub _index: String,
    pub _id: String,
    pub _score: f64,
    pub _source: Value,
}

#[derive(Serialize, Debug, Clone)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
    pub status: u16,
}

#[derive(Serialize, Debug, Clone)]
pub struct ErrorDetails {
    pub root_cause: Vec<ErrorCause>,
    pub r#type: String,
    pub reason: String,
    pub index_uuid: Option<String>,
    pub index: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ErrorCause {
    pub r#type: String,
    pub reason: String,
    pub index_uuid: Option<String>,
    pub index: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ClusterHealthResponse {
    pub cluster_name: String,
    pub status: String,
    pub timed_out: bool,
    pub number_of_nodes: u32,
    pub number_of_data_nodes: u32,
    pub active_primary_shards: u32,
    pub active_shards: u32,
    pub relocating_shards: u32,
    pub initializing_shards: u32,
    pub unassigned_shards: u32,
    pub delayed_unassigned_shards: u32,
    pub number_of_pending_tasks: u32,
    pub number_of_in_flight_fetch: u32,
    pub task_max_waiting_in_queue_millis: u32,
    pub active_shards_percent_as_number: f32,
}

#[derive(Serialize, Clone)]
pub struct RefreshResponse {
    pub _shards: ShardsInfo,
}

#[derive(Serialize, Clone)]
pub struct CountResponse {
    pub count: usize,
    pub _shards: ShardsInfo,
}

pub fn create_error_response(status: u16, error_type: &str, reason: &str) -> ErrorResponse {
    let cause = ErrorCause {
        r#type: error_type.to_string(),
        reason: reason.to_string(),
        index_uuid: None,
        index: None,
    };
    ErrorResponse {
        error: ErrorDetails {
            root_cause: vec![cause],
            r#type: error_type.to_string(),
            reason: reason.to_string(),
            index_uuid: None,
            index: None,
        },
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn should_serialize_count_response() {
        let resp = CountResponse {
            count: 5,
            _shards: ShardsInfo::default(),
        };
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(serialized.contains(r#""count":5"#));
    }

    #[test]
    fn should_serialize_search_response_with_aggregations() {
        let mut aggs = HashMap::new();
        aggs.insert(
            "colors".to_string(),
            AggregationBuckets {
                buckets: vec![BucketResponse {
                    key: json!("red"),
                    doc_count: 10,
                }],
            },
        );

        let resp = SearchResponse {
            took: 10,
            timed_out: false,
            _shards: ShardsInfo::default(),
            hits: HitsMetadata {
                total: TotalHits {
                    value: 1,
                    relation: "eq".to_string(),
                },
                max_score: None,
                hits: vec![],
            },
            aggregations: Some(aggs),
        };

        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(serialized.contains(r#""aggregations":{"colors":{"buckets":[{"#));
    }
}
