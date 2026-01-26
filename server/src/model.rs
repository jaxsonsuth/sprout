use std::error::Error;
use std::num::NonZeroU32;

use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;

pub fn model_init(model_path: &str) -> Result<(LlamaModel, LlamaBackend), Box<dyn Error>> {
    let backend = LlamaBackend::init()?;
    let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())?;
    Ok((model, backend))
}

pub fn context_init<'a>(
    context_size: u32,
    model: &'a LlamaModel,
    backend: &'a LlamaBackend,
) -> Result<LlamaContext<'a>, Box<dyn Error>> {
    let context = model.new_context(
        backend,
        LlamaContextParams::default().with_n_ctx(NonZeroU32::new(context_size)),
    )?;
    Ok(context)
}
