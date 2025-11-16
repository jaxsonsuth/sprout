import time
import requests
import json
from llama_cpp.server.app import create_app
from llama_cpp.server.settings import Settings
import uvicorn
import threading

server_ready = False

def start_server():
    global server_ready
    settings = Settings(
        model="models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf",
        n_ctx=2048,
        n_gpu_layers=-1,
        verbose=False,
    )
    app = create_app(settings)
    server_ready = True
    uvicorn.run(app, host="127.0.0.1", port=8080, log_level="error")

def get_completion_time(prompt, max_tokens=50):
    """Measure time to get completion"""
    start = time.time()
    
    response = requests.post(
        "http://127.0.0.1:8080/v1/completions",
        json={
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.2,
            "stream": False,  # Non-streaming for accurate timing
        },
        timeout=30
    )
    
    elapsed = time.time() - start
    data = response.json()
    
    tokens_generated = 0
    if 'usage' in data:
        tokens_generated = data['usage'].get('completion_tokens', 0)
    
    return elapsed, tokens_generated

def benchmark():
    print("\n" + "="*60)
    print("PREFIX CACHING BENCHMARK")
    print("="*60)
    
    # Test 1: Fresh prompt (no cache)
    print("\n[Test 1] Fresh prompt (cold start, no cache)")
    prompt1 = "def fibonacci(n):"
    time1, tokens1 = get_completion_time(prompt1)
    print(f"  Prompt: '{prompt1}'")
    print(f"  Time: {time1:.3f}s")
    print(f"  Tokens: {tokens1}")
    print(f"  Speed: {tokens1/time1:.1f} tok/s")
    
    # Wait a moment
    time.sleep(0.5)
    
    # Test 2: Same prompt (should hit cache)
    print("\n[Test 2] Same prompt again (should use prefix cache)")
    time2, tokens2 = get_completion_time(prompt1)
    print(f"  Prompt: '{prompt1}'")
    print(f"  Time: {time2:.3f}s")
    print(f"  Tokens: {tokens2}")
    print(f"  Speed: {tokens2/time2:.1f} tok/s")
    speedup = time1 / time2 if time2 > 0 else 0
    print(f"  Speedup: {speedup:.2f}x faster")
    
    time.sleep(0.5)
    
    # Test 3: Extended prompt (prefix match)
    print("\n[Test 3] Extended prompt - one char added (prefix cache hit)")
    prompt2 = "def fibonacci(n):\n"
    time3, tokens3 = get_completion_time(prompt2)
    print(f"  Prompt: '{prompt2.strip()}'\\n")
    print(f"  Time: {time3:.3f}s")
    print(f"  Tokens: {tokens3}")
    print(f"  Speed: {tokens3/time3:.1f} tok/s")
    
    time.sleep(0.5)
    
    # Test 4: Incremental typing simulation
    print("\n[Test 4] Incremental typing simulation")
    base = "def fibonacci(n):"
    
    for i, char in enumerate(["\n", " ", " ", " ", " "]):
        base += char
        start = time.time()
        _, tokens = get_completion_time(base, max_tokens=20)
        elapsed = time.time() - start
        print(f"  Step {i+1}: Added '{repr(char)}' -> {elapsed:.3f}s ({tokens} tokens)")
    
    # Test 5: Different prompt (cache miss)
    print("\n[Test 5] Completely different prompt (cache miss)")
    prompt3 = "def factorial(n):"
    time5, tokens5 = get_completion_time(prompt3)
    print(f"  Prompt: '{prompt3}'")
    print(f"  Time: {time5:.3f}s")
    print(f"  Tokens: {tokens5}")
    print(f"  Speed: {tokens5/time5:.1f} tok/s")
    
    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    print(f"Fresh prompt:           {time1:.3f}s")
    print(f"Same prompt (cached):   {time2:.3f}s  ({time1/time2 if time2 > 0 else 0:.1f}x speedup)")
    print(f"Extended prompt:        {time3:.3f}s")
    print(f"Different prompt:       {time5:.3f}s")
    print("\nIf prefix caching is working:")
    print("  - Test 2 should be much faster than Test 1")
    print("  - Test 3 should be similar to Test 2")
    print("  - Test 4 should show consistently fast times")
    print("  - Test 5 should be similar to Test 1 (new cache)")

def main():
    print("Starting llama server...")
    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()
    
    while not server_ready:
        time.sleep(0.1)
    
    print("Waiting for server to fully initialize...")
    time.sleep(3)
    
    print("Server ready! Starting benchmark...\n")
    
    try:
        benchmark()
    except KeyboardInterrupt:
        print("\n\nBenchmark interrupted")
    except Exception as e:
        print(f"\nError: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()
