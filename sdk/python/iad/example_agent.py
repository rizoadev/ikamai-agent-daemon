"""Example: a simple echo agent that handles text-processing tasks."""

import asyncio
import logging

from . import IadAgent

logging.basicConfig(level=logging.INFO)

agent = IadAgent(
    socket_path="/home/user/.iad/run/iad.sock",
    name="echo-agent",
    capabilities=["echo", "text-processing"],
)


@agent.on_task("echo", "text-processing")
async def handle_echo(task_id: str, payload: dict, cancel) -> dict:
    if cancel.is_cancelled():
        return {"cancelled": True}
    text = payload.get("text", "")
    return {"echo": text, "task_id": task_id}


if __name__ == "__main__":
    asyncio.run(agent.run())
