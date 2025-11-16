import threading
import time
import requests
from llama_cpp.server.app import create_app
from llama_cpp.server.settings import Settings
import uvicorn
from prompt_toolkit import Application
from prompt_toolkit.layout import Layout, HSplit, Window
from prompt_toolkit.widgets import TextArea, Label
from prompt_toolkit.key_binding import KeyBindings

# Global state
server_ready = False
user_input = ""
completion_output = ""

def start_server():
    """Start llama server in background thread"""
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
    """Get completion from server"""
    import json
    try:
        response = requests.post(
            "http://127.0.0.1:8080/v1/completions",
            json={
                "prompt": text,
                "max_tokens": 100,
                "stream": True,
                "stop": ["\n\n"]
            },
            stream=True,
            timeout=5
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
                                yield token
                    except json.JSONDecodeError:
                        pass

    except Exception as e:
        yield f"Error: {e}"

def main():
    global user_input, completion_output

    # Start server
    print("Starting llama server...")
    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()

    # Wait for server
    while not server_ready:
        time.sleep(0.1)
    time.sleep(2)  # Give it a moment to fully initialize

    print("Server ready! Building UI...")

    # Create text areas
    input_area = TextArea(
        text="",
        multiline=True,
        scrollbar=True,
        prompt=">>> "
    )

    output_area = TextArea(
        text="",
        multiline=True,
        scrollbar=True,
        read_only=True
    )

    # Layout
    root_container = HSplit([
        Label(text="Input:"),
        input_area,
        Label(text="Completion:"),
        output_area,
    ])

    # Key bindings
    kb = KeyBindings()

    @kb.add('c-c')
    def exit_(event):
        event.app.exit()

    # Debounce timer
    completion_timer = None
    
    def on_text_change(_):
        """Called when input changes"""
        nonlocal completion_timer
        global user_input, completion_output
        user_input = input_area.text
        
        # Cancel previous timer
        if completion_timer:
            completion_timer.cancel()
        
        # Only get completion if there's text and after a short delay
        if not user_input.strip():
            output_area.text = ""
            return
        
        # Debounce: wait 300ms after typing stops
        def delayed_completion():
            def update_completion():
                global completion_output
                completion_output = ""
                output_area.text = "[Generating...]"
                for token in get_completion(user_input):
                    completion_output += token
                    output_area.text = completion_output
                if not completion_output:
                    output_area.text = "[No completion]"
            
            threading.Thread(target=update_completion, daemon=True).start()
        
        completion_timer = threading.Timer(0.3, delayed_completion)
        completion_timer.start()

    input_area.buffer.on_text_changed += on_text_change

    # Run app
    app = Application(
        layout=Layout(root_container),
        key_bindings=kb,
        full_screen=True
    )

    app.run()

if __name__ == "__main__":
    main()