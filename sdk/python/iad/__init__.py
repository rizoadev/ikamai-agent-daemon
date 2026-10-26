"""IKAMAI Agent Daemon — Python SDK.

Implements:
- task 2.13: Basic Python SDK
- Frame protocol: length-prefixed JSON (task 2.2)
- PING/PONG keepalive
- CancellationToken (task 2.R3)

Usage::

    import asyncio
    from iad import IadAgent

    agent = IadAgent(socket_path="/home/user/.iad/run/iad.sock")

    @agent.on_task("text-generation")
    async def handle(task_id, payload, cancel):
        if cancel.is_cancelled():
            return
        return {"result": f"processed: {payload}"}

    asyncio.run(agent.run())
"""

from .agent import IadAgent
from .protocol import CancellationToken, ErrorClass
from .exceptions import IadError, TaskCancelledError

__version__ = "0.1.0"
__all__ = ["IadAgent", "CancellationToken", "ErrorClass", "IadError", "TaskCancelledError"]
