use axum::Json;
use crate::api::responses::{InfoResponse, VersionInfo, ClusterHealthResponse};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_return_info() {
        let response = info().await;
        assert_eq!(response.version.number, "8.10.0");
        assert_eq!(response.tagline, "You Know, for Search");
    }

    #[tokio::test]
    async fn should_return_green_cluster_health() {
        let response = cluster_health().await;
        assert_eq!(response.status, "green");
        assert_eq!(response.number_of_nodes, 1);
    }
}