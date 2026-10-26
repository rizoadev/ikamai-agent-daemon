#!/usr/bin/env python3
"""Designer Agent — generates HTML/CSS designs."""
import sys, os, asyncio, datetime
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from base_agent import Agent

agent = Agent("designer-agent", ["design", "html-design", "css", "ui"])

@agent.on("design", "html-design", "css", "ui")
async def design(tid, payload):
    msg   = payload.get("message", payload.get("prompt", "landing page"))
    title = payload.get("title", "My App")
    color = payload.get("color", "#6366f1")

    html = f'''<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <title>{title}</title>
  <style>
    :root {{ --brand: {color}; }}
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: 'Segoe UI', system-ui, sans-serif;
      background: linear-gradient(135deg, #0f0c29 0%, #302b63 50%, #24243e 100%);
      min-height: 100vh; display: flex; align-items: center;
      justify-content: center; color: #fff;
    }}
    .hero {{ text-align: center; padding: 4rem 2rem; max-width: 680px;
             animation: up .7s ease both; }}
    @keyframes up {{ from{{opacity:0;transform:translateY(40px)}} to{{opacity:1;transform:none}} }}
    .badge {{ display:inline-block; border:1px solid rgba(255,255,255,.3);
              border-radius:999px; padding:.3rem 1rem; font-size:.8rem;
              letter-spacing:.1em; text-transform:uppercase; margin-bottom:1.5rem; }}
    h1 {{ font-size: clamp(2rem,6vw,3.5rem); font-weight:800; line-height:1.1;
          background: linear-gradient(90deg,#fff,var(--brand));
          -webkit-background-clip:text; -webkit-text-fill-color:transparent;
          margin-bottom:1rem; }}
    p  {{ font-size:1.1rem; line-height:1.8; color:rgba(255,255,255,.75);
          margin-bottom:2.5rem; }}
    .btn {{ display:inline-block; background:var(--brand); color:#fff;
             text-decoration:none; padding:.9rem 2.5rem; border-radius:999px;
             font-weight:700; transition:.2s; box-shadow:0 4px 24px rgba(99,102,241,.4); }}
    .btn:hover {{ transform:translateY(-3px); box-shadow:0 8px 32px rgba(99,102,241,.5); }}
    .meta {{ margin-top:3rem; font-size:.75rem; color:rgba(255,255,255,.3); }}
  </style>
</head>
<body>
  <div class="hero">
    <div class="badge">✨ {title}</div>
    <h1>{msg}</h1>
    <p>Designed automatically by IAD Designer Agent. No human touched this CSS.</p>
    <a href="#" class="btn">Get Started →</a>
    <p class="meta">Generated {datetime.date.today()} · IAD Designer Agent v1.0</p>
  </div>
</body>
</html>'''

    out = payload.get("output", f"/tmp/design-{tid[:8]}.html")
    with open(out, "w") as f: f.write(html)
    print(f"[designer] ✅ wrote {len(html)}b → {out}", flush=True)
    return {"file": out, "bytes": len(html), "title": title, "agent": "designer-agent"}

if __name__ == "__main__":
    asyncio.run(agent.run())
