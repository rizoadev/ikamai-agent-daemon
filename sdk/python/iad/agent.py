"""IadAgent — connects to the daemon and handles tasks."""

import asyncio
import logging
from typing import Any, Callable, Coroutine, Dict, Optional

from .exceptions import IadError, TaskCancelledError
from .protocol import CancellationToken, ErrorClass, read_frame, write_frame

logger = logging.getLogger("iad")


class IadAgent:
    """Connect to IAD daemon and handle tasks.

    Args:
        socket_path: Path to daemon Unix socket.
        name: Agent name (must match manifest).
        capabilities: List of capability strings.
        heartbeat_interval: Seconds between PING frames.
    """

    def __init__(
        self,
        socket_path: str = "/home/user/.iad/run/iad.sock",
        name: str = "python-agent",
        capabilities: Optional[list] = None,
        heartbeat_interval: float = 15.0,
    ):
        self.socket_path = socket_path
        self.name = name
        self.capabilities = capabilities or []
        self.heartbeat_interval = heartbeat_interval
        self._handlers: Dict[str, Callable] = {}
        self._cancel_tokens: Dict[str, CancellationToken] = {}
        self._ping_seq = 0

    def on_task(self, *capabilities: str):
        """Decorator to register a task handler for given capabilities."""
        def decorator(fn: Callable):
            for cap in capabilities:
                self._handlers[cap] = fn
            return fn
        return decorator

    async def run(self):
        """Connect to daemon and start processing tasks."""
        logger.info(f"connecting to daemon at {self.socket_path}")
        reader, writer = await asyncio.open_unix_connection(self.socket_path)
        logger.info(f"connected, advertising capabilities: {self.capabilities}")

        # Advertise capabilities.
        await write_frame(writer, {
            "type": "CAPABILITIES",
            "caps": self.capabilities,
        })

        # Start heartbeat and message handler concurrently.
        await asyncio.gather(
            self._heartbeat_loop(writer),
            self._recv_loop(reader, writer),
        )

    async def _heartbeat_loop(self, writer: asyncio.StreamWriter):
        while True:
            await asyncio.sleep(self.heartbeat_interval)
            self._ping_seq += 1
            await write_frame(writer, {"type": "PONG", "seq": self._ping_seq})

    async def _recv_loop(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
        while True:
            frame = await read_frame(reader)
            msg_type = frame.get("type")

            if msg_type == "ASSIGN_TASK":
                task_id = frame["task_id"]
                payload = frame["payload"]
                asyncio.create_task(self._handle_task(task_id, payload, writer))

            elif msg_type == "CANCEL_TASK":
                task_id = frame["task_id"]
                if task_id in self._cancel_tokens:
                    logger.info(f"cancelling task {task_id}")
                    self._cancel_tokens[task_id].cancel()

            elif msg_type == "PING":
                await write_frame(writer, {"type": "PONG", "seq": frame.get("seq", 0)})

            elif msg_type == "SHUTDOWN":
                logger.info("daemon requested shutdown")
                break

    async def _handle_task(self, task_id: str, payload: Any, writer: asyncio.StreamWriter):
        # Find matching handler by capability in payload.
        handler = self._find_handler(payload)
        if handler is None:
            await write_frame(writer, {
                "type":        "TASK_FAILED",
                "task_id":     task_id,
                "error":       "no handler for task capabilities",
                "error_class": ErrorClass.FATAL,
                "retryable":   False,
            })
            return

        cancel = CancellationToken()
        self._cancel_tokens[task_id] = cancel

        try:
            result = await handler(task_id, payload, cancel)
            await write_frame(writer, {
                "type":    "TASK_COMPLETE",
                "task_id": task_id,
                "result":  result,
            })
        except TaskCancelledError:
            logger.info(f"task {task_id} cancelled by handler")
            # No response needed — daemon already knows.
        except Exception as e:
            logger.exception(f"task {task_id} failed: {e}")
            await write_frame(writer, {
                "type":        "TASK_FAILED",
                "task_id":     task_id,
                "error":       str(e),
                "error_class": ErrorClass.TRANSIENT,
                "retryable":   True,
            })
        finally:
            self._cancel_tokens.pop(task_id, None)

    def _find_handler(self, payload: Any) -> Optional[Callable]:
        if isinstance(payload, dict) and "capability" in payload:
            cap = payload["capability"]
            if cap in self._handlers:
                return self._handlers[cap]
        # Fall back to first registered handler.
        if self._handlers:
            return next(iter(self._handlers.values()))
        return None
