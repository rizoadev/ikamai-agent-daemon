#!/usr/bin/env python3
"""Reviewer Agent — reviews and validates output from other agents."""
import sys, os, asyncio, datetime, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from base_agent import Agent

agent = Agent("reviewer-agent", ["review", "validate", "qa", "check"])

@agent.on("review", "validate", "qa", "check")
async def review(tid, payload):
    msg      = payload.get("message", "")
    files    = payload.get("files", [])
    results  = payload.get("results", {})
    findings = []
    score    = 100

    # Review each file if provided
    for fpath in files:
        if not os.path.exists(fpath):
            findings.append({"file": fpath, "severity": "error", "msg": "File not found"})
            score -= 20
            continue
        content = open(fpath).read()
        size    = len(content)
        issues  = []

        if fpath.endswith(".html"):
            if "<!DOCTYPE" not in content: issues.append("Missing DOCTYPE")
            if "<title>" not in content:    issues.append("Missing <title>")
            if "viewport" not in content:   issues.append("Missing viewport meta")
            if size < 200:                  issues.append("HTML suspiciously short")
        elif fpath.endswith((".js", ".ts")):
            if "console.log" in content: issues.append("console.log found (should use logger)")
            if size < 50:                issues.append("Code suspiciously short")
        elif fpath.endswith(".py"):
            if size < 50:                issues.append("Code suspiciously short")

        if issues:
            score -= len(issues) * 5
            findings.append({"file": fpath, "size_bytes": size, "issues": issues, "severity": "warning"})
        else:
            findings.append({"file": fpath, "size_bytes": size, "issues": [], "severity": "ok"})

    status = "approved" if score >= 70 else "needs_revision" if score >= 40 else "rejected"
    report = {
        "reviewer":    "reviewer-agent",
        "reviewed_at": datetime.datetime.now().isoformat(),
        "score":       max(0, score),
        "status":      status,
        "findings":    findings,
        "summary":     f"Review complete. Score: {max(0,score)}/100. Status: {status}.",
        "task_context": msg,
    }

    out = f"/tmp/review-{tid[:8]}.json"
    with open(out, "w") as f: json.dump(report, f, indent=2)
    print(f"[reviewer] ✅ score={score} status={status} → {out}", flush=True)
    return report

if __name__ == "__main__":
    asyncio.run(agent.run())
