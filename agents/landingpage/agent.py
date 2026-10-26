#!/usr/bin/env python3
"""
IAD Landing Page Agent
Menerima task bikin HTML landing page, return hasil filenya.
"""
import asyncio
import json
import os
import struct
import sys
import textwrap
import datetime

SOCKET_PATH = os.path.expanduser("~/.iad/run/iad.sock")
MAX_FRAME   = 4 * 1024 * 1024

async def write_frame(writer, msg: dict):
    payload = json.dumps(msg).encode()
    writer.write(struct.pack("<I", len(payload)))
    writer.write(payload)
    await writer.drain()

async def read_frame(reader) -> dict:
    hdr  = await reader.readexactly(4)
    size = struct.unpack("<I", hdr)[0]
    data = await reader.readexactly(size)
    return json.loads(data)

def build_landing_page(title: str, headline: str, subtext: str, cta: str) -> str:
    return textwrap.dedent(f"""\
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8"/>
      <meta name="viewport" content="width=device-width,initial-scale=1"/>
      <title>{title}</title>
      <style>
        *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
          font-family: 'Segoe UI', system-ui, sans-serif;
          background: linear-gradient(135deg, #0f0c29, #302b63, #24243e);
          min-height: 100vh;
          display: flex;
          align-items: center;
          justify-content: center;
          color: #fff;
        }}
        .card {{
          text-align: center;
          padding: 3rem 2rem;
          max-width: 600px;
          animation: fadeUp .6s ease both;
        }}
        @keyframes fadeUp {{
          from {{ opacity:0; transform:translateY(30px) }}
          to   {{ opacity:1; transform:translateY(0) }}
        }}
        .badge {{
          display: inline-block;
          background: rgba(255,255,255,.12);
          border: 1px solid rgba(255,255,255,.2);
          border-radius: 999px;
          padding: .3rem 1rem;
          font-size: .8rem;
          letter-spacing: .1em;
          text-transform: uppercase;
          margin-bottom: 1.5rem;
        }}
        h1 {{
          font-size: clamp(2.5rem, 6vw, 4rem);
          font-weight: 800;
          line-height: 1.1;
          margin-bottom: 1rem;
          background: linear-gradient(90deg, #fff 0%, #a78bfa 100%);
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
        }}
        p {{
          font-size: 1.15rem;
          line-height: 1.7;
          color: rgba(255,255,255,.75);
          margin-bottom: 2.5rem;
        }}
        .cta {{
          display: inline-block;
          background: linear-gradient(90deg, #7c3aed, #4f46e5);
          color: #fff;
          text-decoration: none;
          padding: .9rem 2.5rem;
          border-radius: 999px;
          font-weight: 700;
          font-size: 1rem;
          transition: transform .2s, box-shadow .2s;
          box-shadow: 0 4px 24px rgba(124,58,237,.4);
        }}
        .cta:hover {{ transform: translateY(-2px); box-shadow: 0 8px 32px rgba(124,58,237,.5); }}
        footer {{
          margin-top: 3rem;
          font-size: .8rem;
          color: rgba(255,255,255,.35);
        }}
      </style>
    </head>
    <body>
      <div class="card">
        <div class="badge">✨ Welcome</div>
        <h1>{headline}</h1>
        <p>{subtext}</p>
        <a href="#" class="cta">{cta}</a>
        <footer>Built by IAD Landing Page Agent &bull; {datetime.date.today()}</footer>
      </div>
    </body>
    </html>
    """)

async def handle_task(task_id: str, payload: dict) -> dict:
    print(f"[agent] handling task {task_id}", flush=True)
    print(f"[agent] payload: {payload}", flush=True)

    title    = payload.get("title",    "Hello World")
    headline = payload.get("headline", "Hello, World! 👋")
    subtext  = payload.get("subtext",  "This landing page was built automatically by an IAD agent.")
    cta      = payload.get("cta",      "Get Started")
    out_path = payload.get("output",   "/tmp/iad-landing.html")

    html = build_landing_page(title, headline, subtext, cta)
    os.makedirs(os.path.dirname(out_path) if os.path.dirname(out_path) else ".", exist_ok=True)
    with open(out_path, "w") as f:
        f.write(html)

    print(f"[agent] ✅ wrote {len(html)} bytes → {out_path}", flush=True)
    return {
        "status":     "ok",
        "output_file": out_path,
        "bytes":       len(html),
        "title":       title,
        "message":     f"Landing page '{title}' created successfully at {out_path}",
    }

async def run():
    print(f"[agent] connecting to {SOCKET_PATH}", flush=True)
    reader, writer = await asyncio.open_unix_connection(SOCKET_PATH)
    print("[agent] connected", flush=True)

    # Advertise capabilities
    await write_frame(writer, {
        "type": "CAPABILITIES",
        "caps": ["landingpage", "html-generation", "web-build"],
    })
    print("[agent] capabilities advertised", flush=True)

    # Heartbeat loop
    ping_seq = 0
    async def heartbeat():
        nonlocal ping_seq
        while True:
            await asyncio.sleep(15)
            ping_seq += 1
            await write_frame(writer, {"type": "PONG", "seq": ping_seq})

    asyncio.create_task(heartbeat())

    # Main recv loop
    while True:
        frame = await read_frame(reader)
        ft = frame.get("type")

        if ft == "ASSIGN_TASK":
            task_id = frame["task_id"]
            payload = frame["payload"]
            try:
                result = await handle_task(task_id, payload)
                await write_frame(writer, {
                    "type":    "TASK_COMPLETE",
                    "task_id": task_id,
                    "result":  result,
                })
                print(f"[agent] task {task_id} → COMPLETE", flush=True)
            except Exception as e:
                await write_frame(writer, {
                    "type":        "TASK_FAILED",
                    "task_id":     task_id,
                    "error":       str(e),
                    "error_class": "transient",
                    "retryable":   True,
                })
                print(f"[agent] task {task_id} → FAILED: {e}", flush=True)

        elif ft == "PING":
            await write_frame(writer, {"type": "PONG", "seq": frame.get("seq", 0)})

        elif ft == "CANCEL_TASK":
            print(f"[agent] task {frame['task_id']} cancelled", flush=True)

        elif ft == "SHUTDOWN":
            print("[agent] shutdown requested", flush=True)
            break

if __name__ == "__main__":
    asyncio.run(run())
