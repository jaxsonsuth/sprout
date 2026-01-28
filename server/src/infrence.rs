use axum::Json;
use axum::extract::{Path, State};
use axum::http::response;
use llama_cpp_2::context;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::Special::Tokenize;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::model::context_init;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CompletionRequest {
    text: String,
}
#[derive(Serialize)]
pub struct CompletionResponse {
    session_id: String,
    text: String,
}

pub fn completion_handler(
    Path(session_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<CompletionRequest>,
) -> Json<CompletionResponse> {
    let response_text = {
        let context_size: u32 = 2048;

        let mut context = context_init(context_size, &state.model, &state.backend).unwrap();
        let tokens = state
            .model
            .str_to_token(&request.text, AddBos::Always)
            .unwrap();
        let mut batch = LlamaBatch::new(512, 1);

        for (pos, token) in tokens.iter().enumerate() {
            if pos == tokens.len() - 1 {
                batch.add(*token, pos as i32, &[0], true).unwrap();
            } else {
                batch.add(*token, pos as i32, &[0], false).unwrap();
            }
        }

        context.decode(&mut batch).unwrap();

        let token = context.token_data_array().sample_token_greedy();
        let word = state.model.token_to_str(token, Tokenize).unwrap();
        request.text + &word
    };
    Json(CompletionResponse {
        session_id: session_id.to_string(),
        text: response_text,
    })
}
