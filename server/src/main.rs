mod model;

use axum::extract::State;
use axum::{Json, Router, routing::get};
use llama_cpp_2::context::{self, LlamaContext, kv_cache};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use serde::Serialize;
use std::time::Duration;
use std::vec;
use std::{sync::Arc, time::Instant};
use uuid::Uuid;

use crate::model::{context_init, model_init};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    up_time: u64,
}

#[derive(Serialize)]
struct SessionResponse {
    session_id: String,
}

struct AppState {
    start_time: Instant,
    model: LlamaModel,
    backend: LlamaBackend,
}

impl AppState {
    fn up_time(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

struct Session {
    session_id: Uuid,
    kv_cache: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let model_path = "../models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf";

    let Ok((model, backend)) = model_init(model_path) else {
        panic!()
    };

    let state = Arc::new(AppState {
        start_time: Instant::now(),
        model: model,
        backend: backend,
    });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/create_session", get(create_session_handler))
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

async fn create_session_handler() -> Json<SessionResponse> {
    let kv_cache: Vec<u8> = vec![];

    let session = Arc::new(Session {
        session_id: Uuid::new_v4(),
        kv_cache: kv_cache,
    });

    Json(SessionResponse {
        session_id: session.session_id.to_string(),
    })
}
