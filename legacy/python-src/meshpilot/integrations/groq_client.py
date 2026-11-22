"""Groq API: LLM (Llama), Whisper STT, PlayAI TTS. https://console.groq.com"""
from __future__ import annotations
import base64
import logging
from typing import Any

import httpx

from meshpilot.config import config

log = logging.getLogger("meshpilot.groq")
BASE = "https://api.groq.com/openai/v1"

class GroqClient:
    def __init__(self) -> None:
        self.api_key = config.env.groq_api_key
        self.model = config.get("llm", "groq", "model", default="llama-3.3-70b-versatile")
        if not self.api_key:
            raise ValueError("GROQ_API_KEY не задан в .env")

    def _headers(self) -> dict:
        return {"Authorization": f"Bearer {self.api_key}", "Content-Type": "application/json"}

    async def chat(self, messages: list[dict], tools: list[dict] | None = None) -> dict[str, Any]:
        payload: dict[str, Any] = {"model": self.model, "messages": messages, "temperature": 0.7}
        if tools:
            payload["tools"] = [{"type": "function", "function": t} for t in tools]
            payload["tool_choice"] = "auto"
        async with httpx.AsyncClient(timeout=60) as c:
            r = await c.post(f"{BASE}/chat/completions", headers=self._headers(), json=payload)
            r.raise_for_status()
            data = r.json()
        msg = data["choices"][0]["message"]
        tool_calls = []
        for tc in msg.get("tool_calls") or []:
            tool_calls.append({
                "id": tc["id"],
                "name": tc["function"]["name"],
                "arguments": tc["function"]["arguments"],
            })
        return {"content": msg.get("content") or "", "tool_calls": tool_calls}

    async def transcribe(self, wav_bytes: bytes, language: str = "ru") -> str:
        """Whisper large-v3-turbo через Groq."""
        model = config.get("llm", "groq", "whisper_model", default="whisper-large-v3-turbo")
        async with httpx.AsyncClient(timeout=60) as c:
            r = await c.post(
                f"{BASE}/audio/transcriptions",
                headers={"Authorization": f"Bearer {self.api_key}"},
                files={"file": ("audio.wav", wav_bytes, "audio/wav")},
                data={"model": model, "language": language, "response_format": "json"},
            )
            r.raise_for_status()
            return r.json().get("text", "")

    async def tts(self, text: str, voice: str = "Fritz-PlayAI") -> bytes:
        """PlayAI TTS через Groq. Возвращает WAV-байты."""
        model = config.get("llm", "groq", "tts_model", default="playai-tts")
        async with httpx.AsyncClient(timeout=60) as c:
            r = await c.post(
                f"{BASE}/audio/speech",
                headers=self._headers(),
                json={"model": model, "voice": voice, "input": text, "response_format": "wav"},
            )
            r.raise_for_status()
            return r.content
