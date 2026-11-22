"""Конфигурация MeshPilot: YAML + .env + pydantic-settings."""
from __future__ import annotations
import os
from pathlib import Path
from typing import Any

import yaml
from pydantic import BaseModel, Field
from pydantic_settings import BaseSettings, SettingsConfigDict

ROOT = Path(__file__).resolve().parents[2]
CONFIG_DIR = ROOT / "config"


class EnvSettings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", extra="ignore")

    groq_api_key: str = ""
    gigachat_client_id: str = ""
    gigachat_client_secret: str = ""
    gigachat_scope: str = "GIGACHAT_API_PERSIST"
    hermes_base_url: str = "http://localhost:11434/v1"
    hermes_api_key: str = "ollama"
    hermes_model: str = "nous-hermes3:8b"
    openai_compat_base_url: str = ""
    openai_compat_api_key: str = ""
    openai_compat_model: str = ""
    meshpilot_voice: str = "jarvis"
    meshpilot_lang: str = "ru"
    meshpilot_wake_word: str = "mesh-pilot"


def load_yaml(name: str) -> dict[str, Any]:
    path = CONFIG_DIR / name
    if not path.exists():
        return {}
    with path.open(encoding="utf-8") as f:
        return yaml.safe_load(f) or {}


class AppConfig:
    """Единая точка доступа к конфигу."""

    def __init__(self) -> None:
        self.env = EnvSettings()
        self.default: dict[str, Any] = load_yaml("default.yaml")
        self.voices: dict[str, Any] = load_yaml("voices.yaml")
        self.zapret: dict[str, Any] = load_yaml("zapret_strategies.yaml")
        # env override для голоса/языка
        if os.getenv("MESHPILOT_VOICE"):
            self.default.setdefault("app", {})["voice"] = os.getenv("MESHPILOT_VOICE")

    def get(self, *keys: str, default: Any = None) -> Any:
        cur: Any = self.default
        for k in keys:
            if not isinstance(cur, dict) or k not in cur:
                return default
            cur = cur[k]
        return cur

    def voice_profile(self, name: str | None = None) -> dict[str, Any]:
        name = name or self.get("app", "voice", default="jarvis")
        profiles = self.voices.get("profiles", {})
        return profiles.get(name, profiles.get("jarvis", {}))

    def voice_names(self) -> list[str]:
        return list(self.voices.get("profiles", {}).keys())


# синглтон для удобства
config = AppConfig()
