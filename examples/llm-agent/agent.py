"""LLM agent — calls OpenAI API for text generation tasks."""
import asyncio
import os
import json
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../sdk/python'))
from iad import IadAgent

OPENAI_KEY = os.environ.get("OPENAI_API_KEY", "")
MODEL      = os.environ.get("MODEL", "gpt-4o-mini")

agent = IadAgent(
    socket_path=os.path.expanduser("~/.iad/run/iad.sock"),
    name="llm-agent",
    capabilities=["text-generation", "summarize", "code-review"],
)

@agent.on_task("text-generation", "summarize", "code-review")
async def handle(task_id: str, payload: dict, cancel) -> dict:
    if cancel.is_cancelled():
        return {"cancelled": True}

    prompt = payload.get("prompt", payload.get("text", ""))
    if not prompt:
        return {"error": "no prompt provided"}

    # Skeleton — replace with actual OpenAI call
    # import openai
    # response = await openai.AsyncClient().chat.completions.create(
    #     model=MODEL, messages=[{"role": "user", "content": prompt}]
    # )
    # return {"output": response.choices[0].message.content}

    return {
        "output": f"[LLM response to: {prompt[:50]}...]",
        "model":  MODEL,
        "task_id": task_id,
        "note": "Replace with real OpenAI call",
    }

if __name__ == "__main__":
    if not OPENAI_KEY:
        print("Warning: OPENAI_API_KEY not set", file=sys.stderr)
    asyncio.run(agent.run())
