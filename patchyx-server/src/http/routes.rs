//! HTTP route definitions.

use axum::{
    extract::State,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;

use crate::config::ServerConfig;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub start_time: std::time::Instant,
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
}

/// Repository info response.
#[derive(Serialize)]
pub struct RepoInfo {
    pub name: String,
    pub channels: Vec<String>,
}

/// List repositories response.
#[derive(Serialize)]
pub struct ReposResponse {
    pub repositories: Vec<RepoInfo>,
}

/// Create the main router with all routes.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/v1/repos", get(list_repos))
        .with_state(state)
}

/// Root endpoint.
async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "Patchyx Pijul Server",
        "version": env!("CARGO_PKG_VERSION"),
        "docs": "/api/v1"
    }))
}

/// Health check endpoint.
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: uptime,
    })
}

/// List all repositories.
async fn list_repos(State(state): State<AppState>) -> impl IntoResponse {
    // TODO: Actually list repos from state.config.repos_dir
    // For now, return empty list
    let repos_dir = &state.config.repos_dir;
    let mut repositories = Vec::new();

    if repos_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(repos_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        repositories.push(RepoInfo {
                            name: name.to_string(),
                            channels: vec!["main".to_string()], // TODO: Read actual channels
                        });
                    }
                }
            }
        }
    }

    Json(ReposResponse { repositories })
}
