use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use uuid::Uuid;

use crate::session::Session;

pub struct AppState {
    pub start_time: Instant,
    pub model: LlamaModel,
    pub backend: LlamaBackend,
    pub sessions: RwLock<HashMap<Uuid, Session>>,
}

impl AppState {
    pub fn up_time(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
