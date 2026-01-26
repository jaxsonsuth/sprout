use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use std::error::Error;
use std::num::NonZeroU32;
use std::time::Instant;
fn main() -> Result<(), Box<dyn Error>> {
    println!("=== KV Cache Serialization Benchmark ===\n");
    // 1. Initialize model
    println!("Loading model...");
    let model_path = "../models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf";
    let backend = LlamaBackend::init()?;
    let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())?;
    let mut context = model.new_context(
        &backend,
        LlamaContextParams::default().with_n_ctx(NonZeroU32::new(2048)),
    )?;
    println!("✓ Model loaded\n");
    // 2. Create realistic test context (simulating ~100 lines of code)
    println!("Processing test context...");
    let test_code = r#"
use std::collections::HashMap;
struct Calculator {
    memory: HashMap<String, f64>,
}
impl Calculator {
    fn new() -> Self {
        Self {
            memory: HashMap::new(),
        }
    }
    
    fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }
    
    fn subtract(&self, a: f64, b: f64) -> f64 {
        a - b
    }
    
    fn multiply(&self, a: f64, b: f64) -> f64 {
        a * b
    }
    
    fn divide(&self, a: f64, b: f64) -> Option<f64> {
        if b == 0.0 {
            None
        } else {
            Some(a / b)
        }
    }
    
    fn store(&mut self, key: String, value: f64) {
        self.memory.insert(key, value);
    }
    
    fn recall(&self, key: &str) -> Option<f64> {
        self.memory.get(key).copied()
    }
}
fn main() {
    let mut calc = Calculator::new();
    let result = calc.add(5.0, 3.0);
    calc.store("last".to_string(), result);
    println!("Result: {}", result);
"#;
    // Process the context
    let tokens = model.str_to_token(test_code, AddBos::Always)?;
    println!("  Test code: {} characters", test_code.len());
    println!("  Tokenized: {} tokens", tokens.len());

    let mut batch = LlamaBatch::new(512, 1);
    for (pos, token) in tokens.iter().enumerate() {
        let is_last = pos == tokens.len() - 1;
        batch.add(*token, pos as i32, &[0], is_last)?;
    }

    let process_start = Instant::now();
    context.decode(&mut batch)?;
    let process_time = process_start.elapsed();
    println!("  Processing time: {:?}", process_time);
    println!("✓ Context processed\n");
    // 3. Measure state size
    println!("Measuring state size...");
    let state_size = context.get_state_size();
    let state_size_mb = state_size as f64 / 1024.0 / 1024.0;
    println!(
        "  State size: {} bytes ({:.2} MB)",
        state_size, state_size_mb
    );
    println!("✓ Size measured\n");
    // 4. Benchmark serialization
    println!("Benchmarking serialization...");
    let mut buffer = vec![0u8; state_size];

    let mut serialization_times = Vec::new();
    for i in 0..10 {
        let start = Instant::now();
        let bytes_written = unsafe { context.copy_state_data(buffer.as_mut_ptr()) };
        let elapsed = start.elapsed();
        serialization_times.push(elapsed);

        if i == 0 {
            println!("  Bytes written: {}", bytes_written);
        }
    }

    let avg_serialization = serialization_times.iter().sum::<std::time::Duration>() / 10;
    let min_serialization = serialization_times.iter().min().unwrap();
    let max_serialization = serialization_times.iter().max().unwrap();

    println!("  Serialization (avg): {:?}", avg_serialization);
    println!("  Serialization (min): {:?}", min_serialization);
    println!("  Serialization (max): {:?}", max_serialization);
    println!("✓ Serialization benchmarked\n");
    // 5. Benchmark deserialization
    println!("Benchmarking deserialization...");

    let mut deserialization_times = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        let bytes_read = unsafe { context.set_state_data(&buffer) };
        let elapsed = start.elapsed();
        deserialization_times.push(elapsed);

        if deserialization_times.len() == 1 {
            println!("  Bytes read: {}", bytes_read);
        }
    }

    let avg_deserialization = deserialization_times.iter().sum::<std::time::Duration>() / 10;
    let min_deserialization = deserialization_times.iter().min().unwrap();
    let max_deserialization = deserialization_times.iter().max().unwrap();

    println!("  Deserialization (avg): {:?}", avg_deserialization);
    println!("  Deserialization (min): {:?}", min_deserialization);
    println!("  Deserialization (max): {:?}", max_deserialization);
    println!("✓ Deserialization benchmarked\n");
    // 6. Verify generation works after deserialization
    println!("Verifying generation after restore...");

    // Generate a few tokens to verify state is correct
    for i in 0..5 {
        let token = context.token_data_array().sample_token_greedy();
        let text = model.token_to_str(token, llama_cpp_2::model::Special::Tokenize)?;
        print!("{}", text);

        let pos = context.kv_cache_seq_pos_max(0);
        let mut batch = LlamaBatch::new(1, 1);
        batch.add(token, pos + 1, &[0], true)?;
        context.decode(&mut batch)?;
    }
    println!("\n✓ Generation verified\n");
    // 7. Summary
    println!("=== SUMMARY ===");
    println!("Model: qwen2.5-coder-1.5b-instruct");
    println!("Test context: {} tokens", tokens.len());
    println!("State size: {:.2} MB", state_size_mb);
    println!("Serialization: {:?} (avg)", avg_serialization);
    println!("Deserialization: {:?} (avg)", avg_deserialization);
    println!("Round-trip: {:?}", avg_serialization + avg_deserialization);

    // Performance assessment
    let round_trip_ms = (avg_serialization + avg_deserialization).as_millis();
    println!("\n=== ASSESSMENT ===");
    if round_trip_ms < 50 {
        println!("✓ EXCELLENT: Round-trip <50ms - perfect for real-time swapping");
    } else if round_trip_ms < 100 {
        println!("✓ GOOD: Round-trip <100ms - acceptable for session swapping");
    } else if round_trip_ms < 200 {
        println!("⚠ ACCEPTABLE: Round-trip <200ms - noticeable but usable");
    } else {
        println!("✗ CONCERNING: Round-trip >200ms - may need optimization");
    }

    // Memory assessment
    println!("\nMemory per session: {:.2} MB", state_size_mb);
    println!("Estimated for 2 sessions: {:.2} MB", state_size_mb * 2.0);
    println!("Estimated for 5 sessions: {:.2} MB", state_size_mb * 5.0);
    println!("Estimated for 10 sessions: {:.2} MB", state_size_mb * 10.0);
    Ok(())
}
