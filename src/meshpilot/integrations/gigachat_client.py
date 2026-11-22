"""GigaChat API (Sber). OAuth + chat/completions. https://developers.sber.ru/
Русский язык, модели GigaChat2-Max / GigaChat-Max. Нужны client_id/secret из .env.
"""
from __future__ import annotations
import logging
import time
from typing import Any

import httpx

from meshpilot.config import config

log = logging.getLogger("meshpilot.gigachat")
AUTH_URL = "https://ngw.devices.sberdevices.ru:9443/api/v2/oauth"
BASE = "https://gigachat.devices.sberdevices.ru/v1"

class GigaChatClient:
    def __init__(self) -> None:
        self.client_id = config.env.gigachat_client_id
        self.client_secret = config.env.gigachat_client_secret
        self.scope = config.env.gigachat_scope
        self.model = config.get("llm", "gigachat", "model", default="GigaChat2-Max")
        self._token: str = ""
        self._token_exp: float = 0
        if not self.client_id or not self.client_secret:
            raise ValueError("GIGACHAT_CLIENT_ID/SECRET не заданы в .env")

    async def _get_token(self) -> str:
        if self._token and time.time() < self._token_exp:
            return self._token
        async with httpx.AsyncClient(timeout=30, verify=False) as c:
            r = await c.post(
                AUTH_URL,
                headers={
                    "Content-Type": "application/x-www-form-urlencoded",
                    "RqUID": str(int(time.time() * 1000)),
                },
                data={"grant_type": "client_credentials", "scope": self.scope},
                auth=(self.client_id, self.client_secret),
            )
            r.raise_for_status()
            data = r.json()
        self._token = data["access_token"]
        self._token_exp = time.time() + int(data.get("expires_in", 1800)) - 60
        return self._token

    async def chat(self, messages: list[dict], tools: list[dict] | None = None) -> dict[str, Any]:
        token = await self._get_token()
        payload: dict[str, Any] = {"model": self.model, "messages": messages, "temperature": 0.7}
        if tools:
            payload["functions"] = tools
            payload["function_call"] = "auto"
        async with httpx.AsyncClient(timeout=60, verify=False) as c:
            r = await c.post(
                f"{BASE}/chat/completions",
                headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
                json=payload,
            )
            r.raise_for_status()
            data = r.json()
        msg = data["choices"][0]["message"]
        tool_calls = []
        # GigaChat возвращает function_call в message
        fc = msg.get("function_call") or {}
        if fc.get("name"):
            import json as _json
            tool_calls.append({
                "id": "gigachat-1",
                "name": fc["name"],
                "arguments": _json.dumps(fc.get("arguments", {}), ensure_ascii=False),
            })
        return {"content": msg.get("content") or "", "tool_calls": tool_calls}
