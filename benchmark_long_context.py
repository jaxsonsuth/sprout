import time
import requests
import threading
from llama_cpp.server.app import create_app
from llama_cpp.server.settings import Settings
import uvicorn

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

def get_completion_time(prompt, max_tokens=30):
    start = time.time()
    response = requests.post(
        "http://127.0.0.1:8080/v1/completions",
        json={
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.2,
            "stream": False,
        },
        timeout=30
    )
    elapsed = time.time() - start
    return elapsed

def benchmark():
    # Create a long context (simulating a real file)
    long_context = """
def quicksort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    return quicksort(left) + middle + quicksort(right)

def binary_search(arr, target):
    left, right = 0, len(arr) - 1
    while left <= right:
        mid = (left + right) // 2
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1

def merge_sort(arr):
    if len(arr) <= 1:
        return arr
    mid = len(arr) // 2
    left = merge_sort(arr[:mid])
    right = merge_sort(arr[mid:])
    return merge(left, right)

def merge(left, right):
    result = []
    i = j = 0
    while i < len(left) and j < len(right):
        if left[i] < right[i]:
            result.append(left[i])
            i += 1
        else:
            result.append(right[j])
            j += 1
    result.extend(left[i:])
    result.extend(right[j:])
    return result

# Now user starts typing a new function:
def fibonacci(n):"""

    print("\n" + "="*60)
    print("LONG CONTEXT PREFIX CACHING BENCHMARK")
    print("="*60)
    print(f"Context length: {len(long_context)} characters (~{len(long_context.split())} tokens)")
    
    # Test 1: Process long context first time
    print("\n[Test 1] Long context - cold start")
    prompt1 = long_context
    time1 = get_completion_time(prompt1)
    print(f"  Time: {time1:.3f}s")
    
    time.sleep(0.5)
    
    # Test 2: Same long context (should be cached)
    print("\n[Test 2] Same long context (cached)")
    time2 = get_completion_time(prompt1)
    print(f"  Time: {time2:.3f}s")
    print(f"  Speedup: {time1/time2:.2f}x faster")
    
    time.sleep(0.5)
    
    # Test 3-7: User types incrementally
    print("\n[Test 3-7] User types character by character")
    base = long_context
    chars_to_add = ["\n", " ", " ", " ", " ", "i", "f"]
    
    for i, char in enumerate(chars_to_add):
        base += char
        t = get_completion_time(base, max_tokens=20)
        print(f"  Added '{char}' -> {t:.3f}s")
    
    print("\n" + "="*60)
    print("ANALYSIS")
    print("="*60)
    print(f"Cold start (long context): {time1:.3f}s")
    print(f"Cached (same context):     {time2:.3f}s ({time1/time2:.1f}x speedup)")
    print(f"\nWith long context, prefix caching should show bigger speedup!")
    print(f"Expected: 2-5x faster on cached requests")
    print(f"Actual:   {time1/time2:.1f}x faster")

def main():
    print("Starting llama server...")
    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()
    
    while not server_ready:
        time.sleep(0.1)
    
    print("Waiting for server to fully initialize...")
    time.sleep(3)
    print("Server ready!\n")
    
    try:
        benchmark()
    except Exception as e:
        print(f"\nError: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()
