"""Hermes Agent backend: Nous Hermes 3/4 через Ollama (OpenAI-совместимый API).

Запуск локально:
  ollama pull nous-hermes3:8b
  ollama serve   # http://localhost:11434/v1

Альтернативы: hermes-4-405b (через OpenRouter), Qwen2.5-32B-Hermes, Hermes-3-Llama-3.1-8B.
Плюсы: офлайн, приватность, function calling, агентные циклы.
"""
from __future__ import annotations
import logging
from typing import Any

import httpx

from meshpilot.config import config

log = logging.getLogger("meshpilot.hermes")

class HermesClient:
    def __init__(self) -> None:
        self.base_url = (config.env.hermes_base_url or "http://localhost:11434/v1").rstrip("/")
        self.api_key = config.env.hermes_api_key or "ollama"
        self.model = config.env.hermes_model or "nous-hermes3:8b"

    async def chat(self, messages: list[dict], tools: list[dict] | None = None) -> dict[str, Any]:
        payload: dict[str, Any] = {"model": self.model, "messages": messages, "temperature": 0.7}
        if tools:
            payload["tools"] = [{"type": "function", "function": t} for t in tools]
            payload["tool_choice"] = "auto"
        async with httpx.AsyncClient(timeout=120) as c:
            r = await c.post(
                f"{self.base_url}/chat/completions",
                headers={"Authorization": f"Bearer {self.api_key}", "Content-Type": "application/json"},
                json=payload,
            )
            r.raise_for_status()
            data = r.json()
        msg = data["choices"][0]["message"]
        tool_calls = []
        for tc in msg.get("tool_calls") or []:
            tool_calls.append({
                "id": tc.get("id", "hermes-1"),
                "name": tc["function"]["name"],
                "arguments": tc["function"]["arguments"],
            })
        return {"content": msg.get("content") or "", "tool_calls": tool_calls}

    async def health(self) -> bool:
        try:
            async with httpx.AsyncClient(timeout=5) as c:
                r = await c.get(f"{self.base_url}/models",
                                headers={"Authorization": f"Bearer {self.api_key}"})
                return r.status_code == 200
        except Exception:
            return False
