import threading
import time
import requests
import json
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
    )
    app = create_app(settings)
    server_ready = True
    uvicorn.run(app, host="127.0.0.1", port=8080, log_level="error")

def get_completion(text):
    response = requests.post(
        "http://127.0.0.1:8080/v1/completions",
        json={
            "prompt": text,
            "max_tokens": 100,
            "stream": True,
            "stop": ["\n\n", "def ", "class "]
        },
        stream=True
    )
    
    for line in response.iter_lines():
        if line:
            line_str = line.decode('utf-8')
            if line_str.startswith("data: ") and line_str != "data: [DONE]":
                try:
                    data = json.loads(line_str[6:])
                    if 'choices' in data and len(data['choices']) > 0:
                        token = data['choices'][0].get('text', '')
                        if token:
                            print(token, end='', flush=True)
                except:
                    pass

def main():
    print("Starting llama server...")
    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()
    
    while not server_ready:
        time.sleep(0.1)
    time.sleep(2)
    
    print("\nServer ready!")
    print("\nType your code prompt (Ctrl+C to quit):")
    print("=" * 50)
    
    try:
        while True:
            user_input = input("\n>>> ")
            if not user_input.strip():
                continue
            
            print("\n[Completion]", end=' ')
            start_time = time.time()
            get_completion(user_input)
            elapsed = time.time() - start_time
            print(f"\n[Done in {elapsed:.2f}s]")
            
    except KeyboardInterrupt:
        print("\n\nExiting...")

if __name__ == "__main__":
    main()
