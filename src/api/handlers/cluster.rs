use crate::api::responses::{ClusterHealthResponse, InfoResponse, VersionInfo};
use axum::Json;
use axum::http::StatusCode;

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

pub async fn ping() -> StatusCode {
    StatusCode::OK
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
    async fn should_return_info_with_correct_version_and_tagline() {
        let response = info().await;
        assert_eq!(response.version.number, "8.10.0");
        assert_eq!(response.version.build_flavor, "default");
        assert_eq!(response.tagline, "You Know, for Search");
        assert_eq!(response.name, "es_fake");
    }

    #[tokio::test]
    async fn should_respond_ok_to_ping_head_request() {
        let status = ping().await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn should_return_green_cluster_health_with_active_shards() {
        let response = cluster_health().await;
        assert_eq!(response.status, "green");
        assert_eq!(response.cluster_name, "docker-cluster");
        assert_eq!(response.number_of_nodes, 1);
        assert_eq!(response.active_shards, 1);
        assert_eq!(response.active_primary_shards, 1);
        assert!(!response.timed_out);
        assert_eq!(response.active_shards_percent_as_number, 100.0);
    }

    #[tokio::test]
    async fn should_have_zero_pending_tasks_in_health_check() {
        let response = cluster_health().await;
        assert_eq!(response.number_of_pending_tasks, 0);
        assert_eq!(response.relocating_shards, 0);
        assert_eq!(response.unassigned_shards, 0);
    }
}
