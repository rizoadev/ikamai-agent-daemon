"""Wire protocol — length-prefixed JSON frames."""

import asyncio
import json
import struct
from enum import Enum
from typing import Any, Dict

MAX_FRAME_SIZE = 4 * 1024 * 1024  # 4 MiB


class ErrorClass(str, Enum):
    TRANSIENT   = "transient"
    RATE_LIMIT  = "rate_limit"
    FATAL       = "fatal"
    TIMEOUT     = "timeout"

    def is_retryable(self) -> bool:
        return self in (self.TRANSIENT, self.RATE_LIMIT, self.TIMEOUT)


class CancellationToken:
    """Passed to task handlers. Check is_cancelled() before long operations."""

    def __init__(self):
        self._cancelled = False
        self._event = asyncio.Event()

    def cancel(self):
        self._cancelled = True
        self._event.set()

    def is_cancelled(self) -> bool:
        return self._cancelled

    async def wait(self):
        await self._event.wait()


async def write_frame(writer: asyncio.StreamWriter, msg: Dict[str, Any]) -> None:
    payload = json.dumps(msg).encode("utf-8")
    if len(payload) > MAX_FRAME_SIZE:
        raise ValueError(f"frame too large: {len(payload)} bytes")
    writer.write(struct.pack("<I", len(payload)))
    writer.write(payload)
    await writer.drain()


async def read_frame(reader: asyncio.StreamReader) -> Dict[str, Any]:
    len_buf = await reader.readexactly(4)
    length = struct.unpack("<I", len_buf)[0]
    if length > MAX_FRAME_SIZE:
        raise ValueError(f"frame too large: {length} bytes")
    payload = await reader.readexactly(length)
    return json.loads(payload.decode("utf-8"))
