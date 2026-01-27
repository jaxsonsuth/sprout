mod model;

use axum::extract::State;
use axum::{Json, Router, routing::get};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use serde::Serialize;
use serde_json::to_string;
use std::collections::HashMap;
use std::sync::RwLock;
use std::vec;
use std::{sync::Arc, time::Instant};
use uuid::Uuid;

use crate::model::model_init;

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

#[derive(Serialize)]
struct SessionsResponse {
    session_ids: Vec<String>,
}

struct AppState {
    start_time: Instant,
    model: LlamaModel,
    backend: LlamaBackend,
    sessions: RwLock<HashMap<Uuid, Session>>,
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
        sessions: RwLock::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/create_session", get(create_session_handler))
        .route("/get_sessions", get(get_all_sessions_handler))
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

async fn create_session_handler(State(state): State<Arc<AppState>>) -> Json<SessionResponse> {
    let session_id = Uuid::new_v4();
    let session = Session {
        session_id,
        kv_cache: vec![],
    };
    state.sessions.write().unwrap().insert(session_id, session);

    Json(SessionResponse {
        session_id: session_id.to_string(),
    })
}

async fn get_all_sessions_handler(State(state): State<Arc<AppState>>) -> Json<SessionsResponse> {
    let session_ids = state
        .sessions
        .read()
        .unwrap()
        .keys()
        .map(|key| key.to_string())
        .collect();

    Json(SessionsResponse { session_ids })
}
