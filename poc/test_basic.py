from llama_cpp import Llama

# Load model
print("Loading model...")
llm = Llama(
    model_path="models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf",
    n_ctx=2048,        # Context window
    n_gpu_layers=-1,   # Use Metal (all layers on GPU)
    verbose=False
)

print("Model loaded! Testing completion...")

# Simple test
output = llm(
    "def fibonacci(n):",
    max_tokens=100,
    stop=["\n\n"],
    echo=False
)

print("\nCompletion:")
print(output['choices'][0]['text'])
