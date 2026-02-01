use axum::Json;
use axum::extract::{Path, State};
use axum::response::Sse;
use axum::response::sse::Event;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::Special::Tokenize;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::token::LlamaToken;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use uuid::Uuid;

use crate::model::context_init;
use crate::state::AppState;

const SYSTEM_PROMPT: &str = "Complete this code, do not reply with an explaination: ";

#[derive(Deserialize)]
pub struct CompletionRequest {
    text: String,
}
#[derive(Serialize)]
pub struct CompletionResponse {
    session_id: String,
    text: String,
}
#[derive(Serialize)]
struct StreamEvent {
    text: String,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[axum::debug_handler]
pub async fn completion_handler(
    Path(session_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<CompletionRequest>,
) -> Json<CompletionResponse> {
    let context_size: u32 = 4096;
    let mut response_text = String::new();
    let prompt = format!("{}{}", SYSTEM_PROMPT, request.text);

    let mut context = context_init(context_size, &state.model, &state.backend).unwrap();
    process_prompt(&state.model, &prompt, &mut context).expect("Failed to process prompt");

    loop {
        let token = get_next_token(&mut context).unwrap();
        if state.model.is_eog_token(token) {
            break;
        } else {
            print!("{}", token);
            response_text.push_str(&state.model.token_to_str(token, Tokenize).unwrap());
        }
    }

    Json(CompletionResponse {
        session_id: session_id.to_string(),
        text: response_text,
    })
}

pub async fn completion_stream_handler(
    Path(session_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<CompletionRequest>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let context_size: u32 = 4096;
    let prompt = format!("{}{}", SYSTEM_PROMPT, request.text);
    let (tx, rx) = mpsc::channel(32);

    let state = Arc::clone(&state);

    tokio::task::spawn_blocking(move || {
        let mut context = context_init(context_size, &state.model, &state.backend).unwrap();
        process_prompt(&state.model, &prompt, &mut context).expect("Failed to process prompt");

        loop {
            let token = get_next_token(&mut context).unwrap();
            let text = state.model.token_to_str(token, Tokenize).unwrap();
            if state.model.is_eog_token(token) {
                tx.blocking_send((text, true));
                break;
            }
            if tx.blocking_send((text, false)).is_err() {
                break;
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(|(token, done)| {
        let event = StreamEvent {
            text: token,
            done: done,
            error: None,
        };
        Ok(Event::default().data(serde_json::to_string(&event).unwrap()))
    });
    Sse::new(stream)
}

fn process_prompt(
    model: &LlamaModel,
    prompt: &str,
    context: &mut LlamaContext,
) -> Result<(), Box<dyn Error>> {
    let tokens = model.str_to_token(prompt, AddBos::Always)?;

    let mut batch = LlamaBatch::new(512, 1);

    for (pos, token) in tokens.iter().enumerate() {
        if pos == tokens.len() - 1 {
            batch.add(*token, pos as i32, &[0], true)?;
        } else {
            batch.add(*token, pos as i32, &[0], false)?;
        }
    }

    context.decode(&mut batch)?;

    Ok(())
}

fn get_next_token(context: &mut LlamaContext) -> Result<LlamaToken, Box<dyn Error>> {
    let new_token = context.token_data_array().sample_token_greedy();
    let pos = context.kv_cache_seq_pos_max(0);

    let mut batch = LlamaBatch::new(1, 1);
    batch.add(new_token, pos + 1, &[0], true)?;

    context.decode(&mut batch)?;

    Ok(new_token)
}
