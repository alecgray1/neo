//! HTTP Router
//!
//! Sets up the axum router with WebSocket endpoint.

use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;

use super::handler::handle_websocket;
use super::state::AppState;

/// Create the application router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // WebSocket endpoint - all communication goes through here
        .route("/ws", get(ws_upgrade))
        // Health check for monitoring/load balancers
        .route("/health", get(health_check))
        // CORS for development
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// WebSocket upgrade handler
async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    clients: usize,
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        clients: state.client_count().await,
    })
}
