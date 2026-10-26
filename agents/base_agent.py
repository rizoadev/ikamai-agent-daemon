"""Base IAD Agent SDK — shared by all agents."""
import asyncio, json, os, struct, sys, traceback
from typing import Callable, Optional

SOCKET_PATH = os.path.expanduser("~/.iad/run/iad.sock")
MAX_FRAME   = 4 * 1024 * 1024

async def _write(writer, msg: dict):
    p = json.dumps(msg).encode()
    writer.write(struct.pack("<I", len(p))); writer.write(p)
    await writer.drain()

async def _read(reader) -> dict:
    h = await reader.readexactly(4)
    n = struct.unpack("<I", h)[0]
    return json.loads(await reader.readexactly(n))

class Agent:
    def __init__(self, name: str, capabilities: list[str]):
        self.name         = name
        self.caps         = capabilities
        self._handlers:   dict[str, Callable] = {}
        self._writer      = None
        self._ping_seq    = 0
        # Bidirectional: agents can send tasks to each other via daemon
        self._result_cbs: dict[str, asyncio.Future] = {}

    def on(self, *caps):
        """Decorator: register handler for capabilities."""
        def decorator(fn):
            for c in caps: self._handlers[c] = fn
            return fn
        return decorator

    async def send_task(self, agent_name: str, payload: dict, timeout: float = 30.0) -> dict:
        """Ask daemon to route a task to another agent and wait for result via REST."""
        import urllib.request
        body = json.dumps({"agent_name": agent_name, "payload": payload}).encode()
        req  = urllib.request.Request(
            "http://127.0.0.1:7700/api/v1/tasks/dispatch",
            data=body, headers={"Content-Type": "application/json"}, method="POST"
        )
        resp = urllib.request.urlopen(req, timeout=timeout)
        return json.loads(resp.read())

    async def _handle(self, task_id: str, payload: dict) -> dict:
        cap = payload.get("capability") or payload.get("type") or ""
        handler = self._handlers.get(cap) or next(iter(self._handlers.values()), None)
        if not handler:
            raise ValueError(f"No handler for capability '{cap}'")
        return await handler(task_id, payload)

    async def run(self):
        print(f"[{self.name}] connecting...", flush=True)
        reader, writer = await asyncio.open_unix_connection(SOCKET_PATH)
        self._writer   = writer
        await _write(writer, {"type": "CAPABILITIES", "caps": self.caps})
        print(f"[{self.name}] connected, caps={self.caps}", flush=True)

        async def heartbeat():
            while True:
                await asyncio.sleep(15)
                self._ping_seq += 1
                if writer: await _write(writer, {"type": "PONG", "seq": self._ping_seq})

        asyncio.create_task(heartbeat())

        while True:
            frame = await _read(reader)
            ft    = frame.get("type")
            if ft == "ASSIGN_TASK":
                tid, payload = frame["task_id"], frame["payload"]
                asyncio.create_task(self._run_task(tid, payload, writer))
            elif ft == "PING":
                await _write(writer, {"type": "PONG", "seq": frame.get("seq", 0)})
            elif ft == "CANCEL_TASK":
                print(f"[{self.name}] task {frame['task_id']} cancelled", flush=True)
            elif ft == "SHUTDOWN":
                print(f"[{self.name}] shutdown", flush=True); break

    async def _run_task(self, tid, payload, writer):
        print(f"[{self.name}] ← task {tid[:16]}… cap={payload.get('type', payload.get('capability','?'))}", flush=True)
        try:
            result = await self._handle(tid, payload)
            await _write(writer, {"type": "TASK_COMPLETE", "task_id": tid, "result": result})
            print(f"[{self.name}] → DONE {tid[:16]}…", flush=True)
        except Exception as e:
            traceback.print_exc()
            await _write(writer, {"type": "TASK_FAILED", "task_id": tid,
                                  "error": str(e), "error_class": "transient", "retryable": True})
            print(f"[{self.name}] → FAIL {tid[:16]}… {e}", flush=True)
