use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct InfoResponse {
    pub name: String,
    pub cluster_name: String,
    pub version: VersionInfo,
    pub tagline: String,
}

#[derive(Serialize)]
pub struct VersionInfo {
    pub number: String,
    pub build_flavor: String,
}

#[derive(Serialize)]
pub struct IndexResponse {
    pub _index: String,
    pub _id: String,
    pub result: String,
    pub _version: u32,
    pub _shards: ShardsInfo,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct SearchResponse {
    pub took: u128,
    pub timed_out: bool,
    pub _shards: ShardsInfo,
    pub hits: HitsMetadata,
}

#[derive(Serialize)]
pub struct HitsMetadata {
    pub total: TotalHits,
    pub max_score: Option<f64>,
    pub hits: Vec<SearchHit>,
}

#[derive(Serialize)]
pub struct TotalHits {
    pub value: usize,
    pub relation: String,
}

#[derive(Serialize)]
pub struct SearchHit {
    pub _index: String,
    pub _id: String,
    pub _score: f64,
    pub _source: Value,
}

#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
    pub status: u16,
}

#[derive(Serialize, Debug)]
pub struct ErrorDetails {
    pub root_cause: Vec<ErrorCause>,
    pub r#type: String,
    pub reason: String,
    pub index_uuid: Option<String>,
    pub index: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ErrorCause {
    pub r#type: String,
    pub reason: String,
    pub index_uuid: Option<String>,
    pub index: Option<String>,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct RefreshResponse {
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