"""Echo agent — reflects task payload back as result."""
import asyncio
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../sdk/python'))
from iad import IadAgent

agent = IadAgent(
    socket_path=os.path.expanduser("~/.iad/run/iad.sock"),
    name="echo-agent",
    capabilities=["echo", "text-processing"],
)

@agent.on_task("echo", "text-processing")
async def handle(task_id: str, payload: dict, cancel) -> dict:
    if cancel.is_cancelled():
        return {"cancelled": True}
    return {
        "task_id": task_id,
        "echo":    payload,
        "agent":   "echo-agent",
    }

if __name__ == "__main__":
    asyncio.run(agent.run())
