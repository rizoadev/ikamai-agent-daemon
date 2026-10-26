# IAD Agent Development Guide

## What is an IAD Agent?

An agent is any process that:
1. Has a YAML manifest (`name`, `version`, `runtime`, `entrypoint`, `capabilities`)
2. Connects to the daemon via Unix socket (optional — for task-aware agents)
3. Processes tasks and returns results

## Manifest Format

```yaml
name: "my-agent"          # Unique name
version: "1.0.0"
runtime: "python"          # bash, python, node, wasm, go, rust
entrypoint: "agent.py"     # Executable path
args: []                   # Extra args
capabilities:              # What tasks this agent can handle
  - "text-generation"
  - "summarize"
env:
  OPENAI_API_KEY: "${secret:OPENAI_KEY}"   # Secret injection
  LOG_LEVEL: "info"
restart: "on_failure"      # never | on_failure | always
max_restarts: 5
```

## Python Agent (recommended)

```python
# agent.py
import asyncio
from iad import IadAgent

agent = IadAgent(
    socket_path="~/.iad/run/iad.sock",
    name="my-agent",
    capabilities=["text-generation", "summarize"],
)

@agent.on_task("text-generation", "summarize")
async def handle(task_id: str, payload: dict, cancel) -> dict:
    if cancel.is_cancelled():
        return {"cancelled": True}

    prompt = payload.get("prompt", "")
    # ... do work ...
    return {"output": f"processed: {prompt}"}

asyncio.run(agent.run())
```

## Node.js Agent

```js
const { IadAgent } = require('iad-sdk');

const agent = new IadAgent({
  socketPath: '/home/user/.iad/run/iad.sock',
  capabilities: ['echo'],
});

agent.onTask('echo')((taskId, payload, cancel) => {
  if (cancel.isCancelled()) return { cancelled: true };
  return { echo: payload.text };
});

agent.run().catch(console.error);
```

## WASM Agent

Compile your code to WASM and export the IAD ABI:

```rust
// Rust WASM agent
#[no_mangle]
pub extern "C" fn iad_init() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn iad_handle(ptr: i32, len: i32) -> i32 {
    let task = read_string(ptr, len);
    // process...
    write_result("{\"done\": true}");
    0
}

#[no_mangle]
pub extern "C" fn iad_result_ptr() -> i32 { /* ... */ }
#[no_mangle]
pub extern "C" fn iad_result_len() -> i32 { /* ... */ }
```

## Simple Bash Agent (no IPC)

```yaml
name: "cleanup-agent"
version: "1.0.0"
runtime: "bash"
entrypoint: "/home/user/scripts/cleanup.sh"
capabilities: ["maintenance"]
restart: "never"
```

No IPC needed — daemon just manages the process lifecycle.

## Capabilities

Capabilities are arbitrary strings. Convention:
- `text-generation` — LLM text generation
- `image-generation` — image generation
- `echo` — simple echo (for testing)
- `code-review` — code review
- `web-scrape` — web scraping
- `maintenance` — system maintenance tasks
