mod api;
mod domain;
mod repository;

use crate::api::handlers;
use crate::repository::store::InMemoryStore;
use axum::{
    Router, middleware,
    routing::{get, post, put},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct AppState {
    pub store: InMemoryStore,
    pub auth_user: String,
    pub auth_password: String,
    pub auth_enabled: bool,
}

#[tokio::main]
async fn main() {
    let password = std::env::var("ELASTIC_PASSWORD").ok();
    let auth_enabled = password.is_some() && !password.as_ref().unwrap().is_empty();

    let state = Arc::new(AppState {
        store: InMemoryStore::new(),
        auth_user: "elastic".to_string(),
        auth_password: password.unwrap_or_default(),
        auth_enabled,
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 9200));

    println!("--- MICRO-ES STARTING ---");
    println!("Listening on: http://{}", addr);

    let app = Router::new()
        .route("/", get(handlers::info))
        .route("/_cluster/health", get(handlers::cluster_health))
        .route("/_bulk", post(handlers::bulk))
        .route("/{index}/_bulk", post(handlers::bulk))
        .route("/{index}/_refresh", post(handlers::refresh))
        .route(
            "/{index}",
            put(handlers::create_index)
                .head(handlers::check_index)
                .delete(handlers::delete_index),
        )
        .route(
            "/{index}/_mapping",
            get(handlers::get_mapping).put(handlers::put_mapping),
        )
        .route("/{index}/_settings", get(handlers::get_settings))
        .route("/{index}/_mappings", get(handlers::get_mapping))
        .route("/{index}/_doc", post(handlers::index_document))
        .route(
            "/{index}/_doc/{id}",
            get(handlers::get_document)
                .put(handlers::index_document_with_id)
                .post(handlers::index_document_with_id)
                .delete(handlers::delete_document),
        )
        .route("/{index}/_update/{id}", post(handlers::update_document))
        .route(
            "/{index}/_search",
            post(handlers::search).get(handlers::search),
        )
        .route(
            "/{index}/_count",
            post(handlers::count).get(handlers::count),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api::auth::basic_auth,
        ))
        .layer(middleware::from_fn(api::logging::debug_log))
        .with_state(state);

    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("--- SHUTTING DOWN ---");
}
