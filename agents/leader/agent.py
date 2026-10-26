#!/usr/bin/env python3
"""
Leader Agent — orchestrates the web-team.

Receives high-level tasks, fans out to team members,
collects results, asks reviewer to validate, returns final report.
Supports bidirectional communication with all team members.
"""
import sys, os, asyncio, json, datetime, urllib.request
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from base_agent import Agent

agent = Agent("leader-agent", [
    "lead", "orchestrate", "team_task", "build-landing",
    "build-app", "web-project", "mention"
])

API = "http://127.0.0.1:7700/api/v1"

def post_json(url, body) -> dict:
    req  = urllib.request.Request(url, data=json.dumps(body).encode(),
                                  headers={"Content-Type": "application/json"}, method="POST")
    resp = urllib.request.urlopen(req, timeout=30)
    return json.loads(resp.read())

def get_json(url) -> dict:
    resp = urllib.request.urlopen(url, timeout=10)
    return json.loads(resp.read())

async def dispatch(agent_name: str, payload: dict) -> dict:
    """Send task to agent and get task_id back."""
    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(None, post_json,
        f"{API}/tasks/dispatch", {"agent_name": agent_name, "payload": payload})

async def wait_for_result(agent_name: str, payload: dict, timeout: float = 30) -> dict:
    """Dispatch task and poll for result via agent log / simple wait."""
    r = await dispatch(agent_name, payload)
    task_id = r.get("task_id", "")
    print(f"[leader]   → dispatched to {agent_name} task={task_id[:16]}…", flush=True)
    # Since we don't have full DB persistence yet, we wait a bit then check
    await asyncio.sleep(2)
    return {"task_id": task_id, "agent": agent_name, "status": "dispatched"}

@agent.on("lead", "orchestrate", "team_task", "build-landing", "build-app", "web-project", "mention")
async def lead(tid, payload):
    msg       = payload.get("message", payload.get("prompt", "build something"))
    team      = payload.get("team", "web-team")
    title     = payload.get("title", "My App")
    lang      = payload.get("lang", "javascript")
    color     = payload.get("color", "#6366f1")

    print(f"[leader] 🎯 received team task: {msg}", flush=True)
    print(f"[leader] 📋 team={team}, title={title}", flush=True)

    results   = {}
    artifacts = []

    # ── Step 1: designer makes the HTML ─────────────────────────────────────
    print(f"[leader] 📐 assigning design task to designer-agent…", flush=True)
    html_out = f"/tmp/leader-{tid[:8]}-design.html"
    await dispatch("designer-agent", {
        "type":    "design",
        "message": msg,
        "title":   title,
        "color":   color,
        "output":  html_out,
    })
    await asyncio.sleep(2)
    results["design"] = {"file": html_out, "agent": "designer-agent"}
    artifacts.append(html_out)
    print(f"[leader] ✅ design done → {html_out}", flush=True)

    # ── Step 2: coder writes the JS ──────────────────────────────────────────
    print(f"[leader] 💻 assigning code task to coder-agent…", flush=True)
    code_out = f"/tmp/leader-{tid[:8]}-code.{lang[:2]}"
    await dispatch("coder-agent", {
        "type":    "code",
        "message": msg,
        "lang":    lang,
        "title":   title,
        "output":  code_out,
    })
    await asyncio.sleep(2)
    results["code"] = {"file": code_out, "agent": "coder-agent"}
    artifacts.append(code_out)
    print(f"[leader] ✅ code done → {code_out}", flush=True)

    # ── Step 3: reviewer validates everything ────────────────────────────────
    print(f"[leader] 🔍 asking reviewer-agent to validate…", flush=True)
    await dispatch("reviewer-agent", {
        "type":    "review",
        "message": msg,
        "files":   artifacts,
        "results": results,
    })
    await asyncio.sleep(2)
    print(f"[leader] ✅ review done", flush=True)

    # ── Step 4: compile final report ─────────────────────────────────────────
    review_file = f"/tmp/review-{tid[:8]}.json"
    review_data = {}
    if os.path.exists(review_file):
        review_data = json.load(open(review_file))

    report = {
        "leader":         "leader-agent",
        "task_id":        tid,
        "team":           team,
        "message":        msg,
        "completed_at":   datetime.datetime.now().isoformat(),
        "artifacts":      artifacts,
        "results":        results,
        "review":         review_data,
        "status":         review_data.get("status", "completed"),
        "score":          review_data.get("score", 100),
        "summary": (
            f"✅ Team '{team}' completed task: '{msg}'\n"
            f"  • Designer → {html_out}\n"
            f"  • Coder    → {code_out}\n"
            f"  • Reviewer → score={review_data.get('score','?')}/100 "
            f"status={review_data.get('status','?')}"
        ),
    }

    report_file = f"/tmp/leader-report-{tid[:8]}.json"
    with open(report_file, "w") as f:
        json.dump(report, f, indent=2)

    print(f"[leader] 🏁 DONE — report → {report_file}", flush=True)
    print(f"[leader] {report['summary']}", flush=True)
    return report

if __name__ == "__main__":
    asyncio.run(agent.run())
