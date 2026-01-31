mod infrence;
mod model;
mod session;
mod state;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::sync::Arc;

use crate::infrence::{completion_handler, completion_stream_handler};
use crate::model::model_init;
use crate::session::{create_session_handler, get_all_sessions_handler};
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    up_time: u64,
}

#[tokio::main]
async fn main() {
    let model_path = "../models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf";
    let Ok((model, backend)) = model_init(model_path) else {
        panic!()
    };
    let state = Arc::new(AppState::new(model, backend));

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/create_session", get(create_session_handler))
        .route("/get_sessions", get(get_all_sessions_handler))
        .route("/compleat/{id}", post(completion_handler))
        .route("/compleat/stream/{id}", post(completion_stream_handler))
        .with_state(state);

    let addr = "127.0.0.1:8000";
    println!("Server Listening on: {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        up_time: state.up_time(),
    })
}
