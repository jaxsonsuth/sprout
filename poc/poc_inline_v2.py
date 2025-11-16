import threading
import time
import requests
import json
import os
from llama_cpp.server.app import create_app
from llama_cpp.server.settings import Settings
import uvicorn
from prompt_toolkit import Application
from prompt_toolkit.layout import Layout, HSplit, Window, FormattedTextControl
from prompt_toolkit.widgets import TextArea, Label
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.formatted_text import merge_formatted_text, FormattedText
from prompt_toolkit.styles import Style

os.environ['LLAMA_LOG_LEVEL'] = '0'

server_ready = False
current_completion = ""

def start_server():
    global server_ready
    settings = Settings(
        model="models/qwen2.5-coder-1.5b-instruct-q5_k_m.gguf",
        n_ctx=2048,
        n_gpu_layers=-1,
        chat_format="chatml",
        verbose=False,
    )
    app = create_app(settings)
    server_ready = True
    uvicorn.run(app, host="127.0.0.1", port=8080, log_level="error")

def get_completion(text, cancel_event):
    try:
        response = requests.post(
            "http://127.0.0.1:8080/v1/completions",
            json={
                "prompt": text,
                "max_tokens": 150,
                "temperature": 0.2,
                "stream": True,
                "stop": ["\n\n", "def ", "class ", "import ", "from "]
            },
            stream=True,
            timeout=10
        )

        for line in response.iter_lines():
            if cancel_event.is_set():
                response.close()
                return
            
            if line:
                line_str = line.decode('utf-8')
                if line_str.startswith("data: "):
                    if line_str == "data: [DONE]":
                        break
                    try:
                        data = json.loads(line_str[6:])
                        if 'choices' in data and len(data['choices']) > 0:
                            token = data['choices'][0].get('text', '')
                            if token:
                                yield token
                    except json.JSONDecodeError:
                        pass
        
        response.close()

    except Exception as e:
        if not cancel_event.is_set():
            yield f"\n[Error: {e}]"

def main():
    global current_completion
    
    print("Starting llama server...")
    server_thread = threading.Thread(target=start_server, daemon=True)
    server_thread.start()

    while not server_ready:
        time.sleep(0.1)
    time.sleep(2)

    print("Server ready! Building UI...")

    # Create main text area
    text_area = TextArea(
        text="def fibonacci(n):",
        multiline=True,
        scrollbar=True,
    )
    
    # Create a window that shows the ghost text
    def get_ghost_text():
        if current_completion and not current_completion.startswith("["):
            # Show first 80 chars of completion in gray
            preview = current_completion[:80].replace('\n', '↵')
            return FormattedText([('class:ghost', f"→ {preview}")])
        return FormattedText([('', '')])
    
    ghost_window = Window(
        content=FormattedTextControl(get_ghost_text),
        height=1,
    )

    status_text = Label(text="Ready")
    help_text = Label(text="[Tab] Accept | [Esc] Reject | [Ctrl+C] Quit | Ghost text shown below cursor")

    root_container = HSplit([
        text_area,
        ghost_window,
        Window(height=1, char='-'),
        status_text,
        help_text,
    ])

    kb = KeyBindings()

    @kb.add('c-c')
    def exit_(event):
        event.app.exit()
    
    @kb.add('tab')
    def accept_completion(event):
        global current_completion
        if current_completion and not current_completion.startswith("["):
            # Append completion to text
            text_area.text = text_area.text + current_completion
            text_area.buffer.cursor_position = len(text_area.text)
            current_completion = ""
            status_text.text = "✓ Accepted"
    
    @kb.add('escape')
    def reject_completion(event):
        global current_completion
        current_completion = ""
        status_text.text = "✗ Rejected"

    completion_timer = None
    cancel_event = threading.Event()

    def on_text_change(_):
        nonlocal completion_timer, cancel_event
        global current_completion
        
        user_input = text_area.text

        # Cancel previous request
        cancel_event.set()
        
        if completion_timer:
            completion_timer.cancel()

        if not user_input.strip():
            current_completion = ""
            status_text.text = "Ready"
            return

        def delayed_completion():
            nonlocal cancel_event
            global current_completion
            
            cancel_event = threading.Event()
            current_cancel = cancel_event
            
            def update_completion():
                global current_completion
                current_completion = ""
                status_text.text = "⏳ Generating..."
                
                start_time = time.time()
                token_count = 0
                completion_text = ""
                
                try:
                    for token in get_completion(user_input, current_cancel):
                        if current_cancel.is_set():
                            status_text.text = "⚠ Cancelled"
                            return
                        token_count += 1
                        completion_text += token
                        current_completion = completion_text
                        app.invalidate()
                    
                    elapsed = time.time() - start_time
                    tokens_per_sec = token_count / elapsed if elapsed > 0 else 0
                    
                    if not current_cancel.is_set():
                        status_text.text = f"✓ Done | {token_count} tokens in {elapsed:.2f}s ({tokens_per_sec:.1f} tok/s)"
                    
                    if not completion_text and not current_cancel.is_set():
                        current_completion = ""
                        status_text.text = "⚠ No output"
                except Exception as e:
                    if not current_cancel.is_set():
                        status_text.text = f"⚠ Error: {str(e)[:50]}"
            
            threading.Thread(target=update_completion, daemon=True).start()

        completion_timer = threading.Timer(0.5, delayed_completion)
        completion_timer.start()
    
    text_area.buffer.on_text_changed += on_text_change

    style = Style.from_dict({
        'ghost': '#666666',  # Gray color for ghost text
    })
    
    app = Application(
        layout=Layout(root_container),
        key_bindings=kb,
        full_screen=True,
        style=style,
    )

    app.run()

if __name__ == "__main__":
    main()
