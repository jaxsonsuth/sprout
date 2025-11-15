# Temporal Code Completion - Project Overview

## The Core Idea

A novel code completion system that combines **progressive streaming**, **temporal navigation**, and **branching completions** to create an interactive, time-aware coding experience.

### What Makes This Different

Unlike existing tools (GitHub Copilot, Cursor, etc.) that show instant, fixed completions:
1. **Progressive Growth**: Completions start small and automatically expand the longer you wait
2. **Time Travel**: Navigate backward/forward through the completion's generation timeline
3. **Branching**: Accept partial completions and continue generation from that point
4. **Stateful Serving**: KV cache reuse makes incremental updates extremely fast

## Key Innovation: This Doesn't Exist

Research of 90+ AI coding tools (industry + academic) found:
- Streaming exists in chat, not inline completions
- Multiple suggestions exist as parallel alternatives, not progressive growth
- Checkpoints exist at file-level in agent modes, not completion-level
- No temporal navigation of completion history

**This combination is genuinely novel.**

---

## Architecture Overview

### Three Main Components

```
┌─────────────────────────────────────────────────────────┐
│                   Neovim Plugin (Lua)                    │
│  - Text change detection                                 │
│  - Virtual text display                                  │
│  - Temporal navigation UI (←/→ keys)                     │
│  - Accept/reject keybindings                             │
└────────────────┬────────────────────────────────────────┘
                 │ HTTP/WebSocket/stdio
┌────────────────┴────────────────────────────────────────┐
│              Stateful LLM Server (Rust)                  │
│  - Session management (per file/buffer)                  │
│  - KV cache persistence between requests                 │
│  - Incremental context updates                           │
│  - Streaming generation with checkpoints                 │
└────────────────┬────────────────────────────────────────┘
                 │ llama.cpp API
┌────────────────┴────────────────────────────────────────┐
│              Local Model (llama.cpp)                     │
│  - DeepSeek Coder / Qwen2.5 Coder                       │
│  - 6.7B-33B parameters                                   │
│  - Q5/Q8 quantization                                    │
└─────────────────────────────────────────────────────────┘
```

---

## Implementation Details

### Phase 1: Stateful LLM Server (Critical Foundation)

**Why This Matters:**
Current tools (Cursor, Copilot) send full context every request. With stateful serving:
- Process only the new character typed (~1 token)
- Reuse KV cache from previous computation
- Sub-10ms latency vs 50-100ms for cloud solutions
- 99% computation reuse

**Technology Stack:**
- **llama.cpp** - C++ inference engine with KV cache control
- **Rust wrapper** - HTTP server with session management
- **vLLM alternative** - For comparison (has built-in prefix caching)

**Core Features:**
```rust
struct Session {
    id: String,
    kv_cache: KVCache,        // Persistent state
    context: Vec<Token>,       // Current context
    last_completion: String,   // For branching
    checkpoints: Vec<Checkpoint>, // For temporal navigation
}

struct Checkpoint {
    timestamp: u64,
    text: String,
    kv_cache_snapshot: KVCache,
}
```

**API Design:**
```
POST /session/new
  → { session_id: "abc123" }

POST /session/update
  { session_id: "abc123", delta: "f" }
  → Stream: { tokens: ["unction", " add", "(a,", " b)"] }

POST /session/cancel
  { session_id: "abc123" }
  → { cancelled: true }

GET /session/checkpoint
  { session_id: "abc123", index: 3 }
  → { text: "...", kv_cache_id: "xyz" }
```

**Key Challenges:**
1. Memory management - KV cache can be large (GB per session)
2. Concurrent sessions - Need efficient cache swapping
3. Cache invalidation - When does context become stale?
4. Checkpoint storage - How many to keep?

---

### Phase 2: Neovim Plugin (UI/UX Layer)

**Core Functionality:**

**1. Autocommands for Text Changes**
```lua
vim.api.nvim_create_autocmd({"TextChangedI", "TextChanged"}, {
  callback = function()
    debounce_completion(300)  -- Wait 300ms after typing stops
  end
})
```

**2. Virtual Text for Completions**
```lua
-- Display ghost text
vim.api.nvim_buf_set_extmark(buf, ns, line, col, {
  virt_text = {{completion_text, "Comment"}},
  virt_text_pos = "inline"
})
```

**3. Temporal Navigation**
```lua
-- Timeline structure
completion_timeline = {
  {time = 0, text = ""},
  {time = 50, text = "unction add("},
  {time = 200, text = "unction add(a, b) {"},
  {time = 500, text = "unction add(a, b) {\n  return a + b;\n}"},
  {time = 1000, text = "... full implementation"}
}

-- Navigate with arrow keys
vim.keymap.set("i", "<Left>", step_back_in_timeline)
vim.keymap.set("i", "<Right>", step_forward_in_timeline)
```

**4. Branching Acceptance**
```lua
function accept_and_branch()
  local cursor_position = get_timeline_position()
  local accepted_text = timeline[cursor_position].text
  
  -- Insert accepted text
  insert_at_cursor(accepted_text)
  
  -- Start new completion from this point
  start_new_completion_branch(accepted_text)
end
```

**Key Bindings:**
- `Tab` - Accept current completion
- `Esc` - Reject/dismiss
- `Ctrl+[` or `←` - Step back in timeline
- `Ctrl+]` or `→` - Step forward in timeline
- Auto-continue on accept (starts new branch)

**Visual Feedback:**
```
Current file:
  1  def fibonacci(n):
  2      if n <= 1:|          ← cursor here
  3  
Completion (ghosted):
          return n
      return fibonacci(n-1) + fibonacci(n-2)

Timeline indicator: [===•====] (50% through generation)
Checkpoint 3/7 | 500ms elapsed
```

---

### Phase 3: Context Management

**Progressive Context Expansion:**

```
t=0ms   (user pauses typing)
├─ Minimal context: Current line + 20 lines before/after
│  → Fast generation starts (~50ms latency)
│  → Display initial completion
│
t=200ms (still idle)
├─ Standard context: Full current file + imports
│  → Continue generation with better context
│  → May refine/extend completion
│
t=500ms (still idle)
└─ Rich context: LSP definitions + related files
   → Full-quality completion
   → May completely rewrite with better understanding
```

**Context Sources (Priority Order):**
1. **Current line** (always included)
2. **Surrounding ±20 lines** (always included)
3. **Full current file** (if < 2000 lines)
4. **Imports/requires** (via tree-sitter parsing)
5. **LSP definitions** (types, function signatures)
6. **Recently edited files** (from vim buffer list)
7. **Related files** (same directory, similar names)

**Smart Truncation:**
- Use tree-sitter to maintain syntactic boundaries
- Prefer complete functions over partial ones
- Include function signatures even if body truncated
- Keep imports/type definitions

---

## Experimental Areas

### 1. Completion Evolution Strategy

**Question:** How should completions grow over time?

**Option A: Iterative Refinement**
```
t=50ms:  "unction add(a, b)"
t=200ms: "unction add(a, b) {\n  return"
t=500ms: "unction add(a, b) {\n  return a + b;\n}"
t=1000ms: "... + docstring + error handling"
```

**Option B: Granularity Expansion**
```
t=50ms:  Complete current line
t=200ms: Complete current function
t=500ms: Complete current class/module
t=1000ms: Generate related helper functions
```

**Option C: Confidence Thresholds**
```
Only show completions when model confidence > threshold
Low wait time → high threshold (only very confident)
Long wait time → lower threshold (show more options)
```

**Experiment:** A/B test with users to find optimal strategy

---

### 2. Temporal Navigation UX

**Question:** How to visualize the timeline?

**Option A: Discrete Checkpoints**
```
[1] [2] [3•] [4] [5]  ← Navigate with arrows
```

**Option B: Continuous Slider**
```
|====•========|  ← Scrub through like video timeline
0ms          1000ms
```

**Option C: Hierarchical Tree**
```
def fibonacci(n):
  ├─ if n <= 1:
  │   ├─ return n  ← Current position
  │   └─ return 1  ← Alternative branch
  └─ return fibonacci(n-1) + fibonacci(n-2)
```

**Experiment:** 
- Eye tracking study to see where users look
- Measure time-to-accept with different UIs
- Cognitive load assessment

---

### 3. KV Cache Management

**Question:** When to invalidate/update cache?

**Strategy A: Strict Prefix Matching**
- Only reuse if new context is exact prefix extension
- Very safe, but less reuse opportunity

**Strategy B: Fuzzy Matching**
- Allow minor edits (typo fixes, whitespace)
- Re-anchor cache to new context
- Risk: Model confusion from inconsistent state

**Strategy C: Semantic Similarity**
- Use embedding distance to detect "close enough" contexts
- Rebuild cache only if semantic drift > threshold
- Most efficient but most complex

**Metrics to Track:**
- Cache hit rate
- Time saved per completion
- Quality degradation (if any)
- Memory usage patterns

---

### 4. Multi-File Awareness

**Question:** How to handle completions that span files?

**Scenario:** User types `import calculate_` and needs function from another file

**Option A: Cross-File Search**
```
1. Detect import statement
2. Search project for matching function names
3. Include relevant file snippets in context
4. Complete with correct import path
```

**Option B: Embedding-Based Retrieval**
```
1. Pre-compute embeddings for all functions
2. Query embedding space for relevant code
3. Inject into context dynamically
4. Cache embeddings, update on file changes
```

**Option C: LSP Integration**
```
1. Query LSP for definitions/references
2. Follow dependency chain
3. Include type information
4. Respect project structure
```

**Experiment:** 
- Measure completion quality with/without cross-file context
- Test performance impact of different retrieval methods
- User study: Do developers prefer explicit vs implicit cross-file suggestions?

---

### 5. Branching Behavior

**Question:** What happens after accepting a partial completion?

**Option A: Simple Continuation**
```
User accepts: "def fibonacci(n):\n    if n <= 1:"
AI continues from that point with fresh generation
Previous timeline discarded
```

**Option B: Tree Exploration**
```
User accepts checkpoint 3 of 7
Create new branch from checkpoint 3
Keep original branch in history (can return to it)
Build tree of possibilities over time
```

**Option C: Hybrid Path**
```
User accepts: "if n <= 1:"
AI generates multiple possible continuations:
  - "return n"
  - "return 1"
  - "raise ValueError"
Show as parallel branches, user picks one
```

**UI Challenge:**
- How to visualize branch tree without overwhelming?
- Keyboard shortcuts for branch navigation?
- Memory limits on branch history?

---

### 6. Model Selection & Optimization

**Question:** What's the optimal model size/quality tradeoff?

**Candidates:**

| Model | Size | Speed | Quality | Notes |
|-------|------|-------|---------|-------|
| Qwen2.5-Coder 0.5B | 350MB | <20ms | Basic | Fast prototyping |
| Qwen2.5-Coder 1.5B | 1GB | 30-50ms | Good | Current test model |
| DeepSeek Coder 6.7B | 4GB | 50-150ms | Very Good | Best balance |
| Qwen2.5-Coder 32B | 20GB | 200-400ms | Excellent | Max quality |

**Hardware Context:** M4 Max with 48GB RAM

**Experiments:**
1. **Latency Tolerance Study**
   - At what latency do users stop waiting?
   - Measure accept rate vs. generation time
   - Find optimal debounce delay

2. **Quality vs Speed Tradeoff**
   - Same prompt to all models
   - Measure: accuracy, relevance, accept rate
   - Cost: latency, memory, power usage

3. **Quantization Impact**
   - Test Q4, Q5, Q6, Q8 versions
   - Measure quality degradation
   - Find sweet spot for code completion

4. **Speculative Decoding**
   - Use 1.5B model to draft
   - Use 32B model to verify/correct
   - Potentially get 32B quality at 6.7B speed

---

### 7. Streaming Speed Tuning

**Question:** How fast should tokens stream?

**Current:** 50ms delay between tokens (artificial)

**Considerations:**
- **Too fast:** User can't read, feels jarring
- **Too slow:** Impatient, switch to different tool
- **Variable speed:** Start slow, accelerate?

**Experiment:**
```python
# Option A: Constant speed
stream_delay = 50ms

# Option B: Accelerating
stream_delay = 100ms → 50ms → 20ms → 10ms

# Option C: Adaptive
if user_is_reading():
    stream_delay = 80ms  # Readable pace
else:
    stream_delay = 20ms  # Fast forward
```

**Metrics:**
- User eye gaze (are they reading?)
- Accept timing (do they wait for full completion?)
- Subjective preference survey

---

### 8. Error Recovery & Edge Cases

**Scenarios to Handle:**

**1. Model Goes Off Track**
```
User types: "def add(a, b):"
Model suggests: "def add(a, b):\n    # This function multiplies..."
```
Solution: Temporal rewind to checkpoint before divergence

**2. Syntax Errors**
```
Model generates invalid Python
```
Solution: 
- Tree-sitter validation before display?
- Show anyway but mark as uncertain?
- Auto-correct obvious mistakes?

**3. Very Long Completions**
```
Model wants to generate 1000+ lines
```
Solution:
- Token limit per time checkpoint
- Soft cap with "continue?" prompt
- Progressive token budget increase

**4. Cache Corruption**
```
KV cache becomes inconsistent with context
```
Solution:
- Checksum validation
- Automatic cache rebuild on mismatch
- Fallback to stateless mode

**5. Rapid Context Switching**
```
User jumps between files quickly
Multiple sessions active
```
Solution:
- Session prioritization/eviction
- Cache LRU policy
- Max concurrent sessions limit

---

## Performance Targets

### Latency Goals

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| First token | <50ms | <20ms |
| Token streaming | 50ms/token | Variable (20-100ms) |
| Cache hit speedup | 10x faster | 50x faster |
| Cancellation | <10ms | <5ms |
| Context gathering | <20ms | <10ms |

### Quality Metrics

| Metric | Measurement Method | Target |
|--------|-------------------|--------|
| Accept rate | % of completions accepted | >40% |
| Partial accept | % accepted after rewind | >20% |
| Edit distance | User edits to completion | <30% |
| Time to accept | Seconds from display | <3s |
| Abandonment | % of completions ignored | <40% |

### Resource Constraints

| Resource | Limit | Reason |
|----------|-------|--------|
| Memory per session | <2GB | Support 10+ concurrent files |
| Total memory | <30GB | Leave room for OS + other apps |
| CPU usage | <50% sustained | Don't thermal throttle |
| Disk cache | <10GB | SSD space is precious |

---

## Development Roadmap

### Milestone 1: Proof of Concept ✓ (COMPLETE)
- [x] Python prototype with Ollama
- [x] Basic streaming UI with prompt_toolkit
- [x] Demonstrated feasibility
- [x] Validated UX concept

### Milestone 2: Stateful Server (4-6 weeks)
- [ ] llama.cpp integration
- [ ] Rust HTTP server
- [ ] Session management
- [ ] KV cache persistence
- [ ] Basic checkpoint system
- [ ] Benchmark: 10x speedup on incremental updates

### Milestone 3: Neovim Plugin Alpha (2-3 weeks)
- [ ] Basic autocommands
- [ ] Virtual text display
- [ ] Server communication
- [ ] Accept/reject keybindings
- [ ] Debounce logic
- [ ] Cancellation support

### Milestone 4: Temporal Navigation (3-4 weeks)
- [ ] Checkpoint recording
- [ ] Timeline data structure
- [ ] Navigation keybindings (←/→)
- [ ] Visual timeline indicator
- [ ] Smooth transitions

### Milestone 5: Branching System (3-4 weeks)
- [ ] Partial acceptance
- [ ] Branch creation
- [ ] New completion from branch point
- [ ] Branch visualization
- [ ] Branch history management

### Milestone 6: Context Enhancement (2-3 weeks)
- [ ] Tree-sitter integration
- [ ] LSP client
- [ ] Multi-file awareness
- [ ] Progressive expansion
- [ ] Smart truncation

### Milestone 7: Polish & Optimization (4-6 weeks)
- [ ] Performance tuning
- [ ] Error handling
- [ ] Configuration system
- [ ] Documentation
- [ ] User testing
- [ ] Bug fixes

**Total Estimated Time:** 4-6 months for full implementation

---

## Research Questions

### User Experience
1. Do users actually want progressive completions?
2. Is temporal navigation intuitive or confusing?
3. At what point do completions become "too long"?
4. Preferred visual style for ghost text?
5. Optimal debounce delay per user?

### Technical Performance
1. What's the actual KV cache hit rate in practice?
2. Memory usage patterns with real codebases?
3. Does progressive context actually improve quality?
4. Optimal checkpoint frequency?
5. Best model for this use case?

### Novel Behaviors
1. How do users explore the timeline?
2. Branching usage patterns?
3. Accept rate vs. traditional completions?
4. Impact on coding speed/quality?
5. Learning curve duration?

---

## Success Criteria

### Minimum Viable Product
- [ ] Completions appear as user types
- [ ] Can navigate backward/forward through timeline
- [ ] Can accept partial completions
- [ ] Faster than starting from scratch
- [ ] Stable for daily use

### Compelling Product
- [ ] Noticeably faster than Copilot
- [ ] Higher accept rate than Copilot
- [ ] Temporal navigation feels natural
- [ ] Works offline
- [ ] Low resource usage

### Research Contribution
- [ ] Novel interaction paradigm validated
- [ ] User study with 20+ developers
- [ ] Published paper or detailed blog post
- [ ] Open source for others to build on
- [ ] Inspiration for future tools

---

## References & Prior Art

### Existing Tools Analyzed
- GitHub Copilot (ghost text, alternatives)
- Cursor (agent mode, checkpoints)
- Windsurf/Cascade (continuous awareness)
- Codeium, Tabnine (traditional completion)
- Claude Code (rewind feature)

### Academic Research
- "Mapping the Design Space of AI Coding Assistants" (Lau & Guo, 2025)
- "Understanding user mental models in AI-driven code completion tools" (IJHCS, 2025)
- Studied 90+ AI coding tools - none have this combo

### Technical Foundations
- llama.cpp - KV cache control
- vLLM - Production inference
- Tree-sitter - Code parsing
- LSP - Language intelligence
- Neovim API - Plugin infrastructure

---

## Getting Started

### Immediate Next Steps
1. Build stateful server with llama.cpp
2. Test KV cache reuse performance
3. Measure actual speedup vs. stateless
4. Prototype Neovim plugin basics
5. User test the core interaction

### Questions to Answer First
- Which model to target? (Recommend: DeepSeek Coder 6.7B)
- Rust or Go for server? (Rust for llama.cpp bindings)
- HTTP or stdio for communication? (HTTP easier to debug)
- How to package/distribute? (Later problem)

---

## License & Collaboration

This is a research project and personal exploration. Consider:
- Open source the final product
- Write detailed blog posts about findings
- Submit to academic venues (CHI, UIST, VL/HCC)
- Collaborate with HCI researchers
- Share novel insights with community

---

**Last Updated:** October 23, 2025
**Status:** Proof of concept complete, moving to stateful server implementation
**Next Milestone:** Stateful LLM server with KV cache persistence
