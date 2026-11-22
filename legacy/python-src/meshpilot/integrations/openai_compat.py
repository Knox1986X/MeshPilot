"""Универсальный OpenAI-совместимый клиент (OpenRouter, Together, LM Studio, vLLM)."""
from __future__ import annotations
from typing import Any

import httpx

from meshpilot.config import config

class OpenAICompatClient:
    def __init__(self) -> None:
        self.base_url = config.env.openai_compat_base_url.rstrip("/")
        self.api_key = config.env.openai_compat_api_key or "sk-"
        self.model = config.env.openai_compat_model or "gpt-4o-mini"
        if not self.base_url:
            raise ValueError("OPENAI_COMPAT_BASE_URL не задан")

    async def chat(self, messages: list[dict], tools: list[dict] | None = None) -> dict[str, Any]:
        payload: dict[str, Any] = {"model": self.model, "messages": messages}
        if tools:
            payload["tools"] = [{"type": "function", "function": t} for t in tools]
        async with httpx.AsyncClient(timeout=60) as c:
            r = await c.post(
                f"{self.base_url}/chat/completions",
                headers={"Authorization": f"Bearer {self.api_key}", "Content-Type": "application/json"},
                json=payload,
            )
            r.raise_for_status()
            data = r.json()
        msg = data["choices"][0]["message"]
        tool_calls = [
            {"id": tc.get("id", "1"), "name": tc["function"]["name"], "arguments": tc["function"]["arguments"]}
            for tc in (msg.get("tool_calls") or [])
        ]
        return {"content": msg.get("content") or "", "tool_calls": tool_calls}
