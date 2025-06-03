pub mod grpc;

use crate::config::ConfigManager;
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tonic::transport::Server;

pub async fn run_server(
    port: u16,
    token: Option<String>,
    config_manager: ConfigManager,
) -> Result<()> {
    // Clone config_manager for both servers
    let http_config = config_manager.clone();
    let grpc_config = config_manager.clone();
    
    // Start gRPC server on port + 1
    let grpc_port = port + 1;
    let grpc_addr = format!("0.0.0.0:{}", grpc_port).parse()?;
    
    let grpc_service = grpc::create_grpc_service(grpc_config);
    
    let grpc_handle = tokio::spawn(async move {
        tracing::info!("gRPC server listening on {}", grpc_addr);
        Server::builder()
            .add_service(grpc_service)
            .serve(grpc_addr)
            .await
    });

    // Start HTTP server
    let app_state = Arc::new(AppState {
        config_manager: http_config,
        auth_token: token,
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/v1/query", post(query_handler))
        .route("/v1/index/status", get(index_status_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("HTTP server listening on {}", addr);
    
    // Run both servers concurrently
    tokio::select! {
        result = axum::serve(listener, app) => {
            result?;
        }
        result = grpc_handle => {
            result??;
        }
    }

    Ok(())
}

#[derive(Clone)]
struct AppState {
    config_manager: ConfigManager,
    auth_token: Option<String>,
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "buildli",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn query_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({
        "message": "Query endpoint not yet implemented",
        "query": payload
    })))
}

async fn index_status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({
        "status": "ok",
        "total_files": 0,
        "indexed_files": 0,
        "total_chunks": 0,
        "last_updated": "never"
    })))
}