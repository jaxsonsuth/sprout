use axum::Json;
use axum::extract::State;
use serde::Serialize;
use std::sync::Arc;
use std::vec;
use uuid::Uuid;

use crate::state::AppState;

pub struct Session {
    session_id: Uuid,
    kv_cache: Vec<u8>,
}

//Create Session
#[derive(Serialize)]
pub struct SessionResponse {
    session_id: String,
}
pub async fn create_session_handler(State(state): State<Arc<AppState>>) -> Json<SessionResponse> {
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

// List all sessions
#[derive(Serialize)]
pub struct SessionsResponse {
    session_ids: Vec<String>,
}
pub async fn get_all_sessions_handler(
    State(state): State<Arc<AppState>>,
) -> Json<SessionsResponse> {
    let session_ids = state
        .sessions
        .read()
        .unwrap()
        .keys()
        .map(|key| key.to_string())
        .collect();

    Json(SessionsResponse { session_ids })
}
