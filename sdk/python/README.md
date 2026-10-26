# IAD Python SDK

Minimal Python SDK for writing agents that connect to the IKAMAI Agent Daemon.

## Install

```bash
pip install ./sdk/python
```

## Usage

```python
import asyncio
from iad import IadAgent

agent = IadAgent(
    socket_path="~/.iad/run/iad.sock",
    name="my-agent",
    capabilities=["text-generation", "summarize"],
)

@agent.on_task("text-generation", "summarize")
async def handle(task_id, payload, cancel):
    if cancel.is_cancelled():
        return None
    text = payload.get("prompt", "")
    return {"output": f"[processed] {text}"}

asyncio.run(agent.run())
```
