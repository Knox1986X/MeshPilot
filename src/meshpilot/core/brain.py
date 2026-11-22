"""Мозг MeshPilot: каскад LLM Groq -> GigaChat -> Hermes (Ollama) -> OpenAI-compat.

Почему так:
- Groq: самый быстрый облачный инференс (Llama 3.3 70B, ~500 tok/s), бесплатный tier, Whisper+TTS в одном API.
- GigaChat: русский язык из коробки, Sber, хорошо для RU-пользователей и гос-сетей.
- Hermes (Nous Hermes 3/4, Qwen-Hermes): локально через Ollama — приватность, офлайн, function calling,
  агентные циклы без облака. Это и есть «Hermes Agent» аналог: свой лёгкий агент поверх Hermes.
- OpenAI-compat: любой провайдер (OpenRouter, Together, LM Studio) как fallback.
"""
from __future__ import annotations
import logging
from typing import Any

from meshpilot.config import config
from meshpilot.integrations.gigachat_client import GigaChatClient
from meshpilot.integrations.groq_client import GroqClient
from meshpilot.integrations.hermes_client import HermesClient
from meshpilot.integrations.openai_compat import OpenAICompatClient

log = logging.getLogger("meshpilot.brain")

class Brain:
    def __init__(self, voice: str | None = None) -> None:
        self.voice = voice or config.get("app", "voice", default="jarvis")
        self.profile = config.voice_profile(self.voice)
        self.system_prompt = self.profile.get("persona", "Ты — MeshPilot, голосовой ассистент.")
        self.providers: list[tuple[str, Any]] = []
        cascade = config.get("llm", "cascade", default=["groq", "gigachat", "hermes"])
        for name in cascade:
            try:
                if name == "groq" and config.env.groq_api_key:
                    self.providers.append(("groq", GroqClient()))
                elif name == "gigachat" and config.env.gigachat_client_id:
                    self.providers.append(("gigachat", GigaChatClient()))
                elif name == "hermes":
                    self.providers.append(("hermes", HermesClient()))
                elif name == "openai_compat" and config.env.openai_compat_base_url:
                    self.providers.append(("openai_compat", OpenAICompatClient()))
            except Exception as e:
                log.warning("provider %s init failed: %s", name, e)
        if not self.providers:
            # хотя бы Hermes (локальный Ollama может подняться позже)
            self.providers.append(("hermes", HermesClient()))
        log.info("Brain providers: %s voice=%s", [p[0] for p in self.providers], self.voice)

    async def chat(self, messages: list[dict], tools: list[dict] | None = None) -> dict[str, Any]:
        """Пробует провайдеры по порядку, возвращает {content, provider, tool_calls}."""
        last_err: Exception | None = None
        for name, client in self.providers:
            try:
                res = await client.chat(messages, tools=tools)
                res["provider"] = name
                return res
            except Exception as e:
                log.warning("provider %s failed: %s", name, e)
                last_err = e
                continue
        raise RuntimeError(f"Все LLM провайдеры недоступны. Последняя ошибка: {last_err}")

    async def complete(self, user_text: str, history: list[dict] | None = None) -> str:
        msgs = [{"role": "system", "content": self.system_prompt}]
        msgs.extend(history or [])
        msgs.append({"role": "user", "content": user_text})
        res = await self.chat(msgs)
        return res.get("content", "")

    def set_voice(self, voice: str) -> None:
        self.voice = voice
        self.profile = config.voice_profile(voice)
        self.system_prompt = self.profile.get("persona", self.system_prompt)
