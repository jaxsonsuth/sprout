use std::error::Error;
use std::num::NonZeroU32;

use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::Special::Tokenize;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::token::LlamaToken;

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

fn main() -> Result<(), Box<dyn Error>> {
    // INIT
    let path = "../models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf";
    let backend = LlamaBackend::init()?;
    let model = LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default())?;
    let mut context = model.new_context(
        &backend,
        LlamaContextParams::default().with_n_ctx(NonZeroU32::new(2048)),
    )?;
    // END INIT

    let prompt = "fn get_next_token(context: &mut LlamaContext) -> Result<LlamaToken, Box<dyn Error>> {let new_token = context.token_data_array().sample_token_greedy();let pos = context.kv_cache_seq_pos_max(0);";

    print!("{}", prompt);

    process_prompt(&model, prompt, &mut context)?;

    loop {
        let token = get_next_token(&mut context)?;
        if model.is_eog_token(token) {
            break;
        } else {
            let text = model.token_to_str(token, Tokenize)?;
            print!("{}", text);
        }
    }

    println!("");
    println!("Done!");
    Ok(())
}
