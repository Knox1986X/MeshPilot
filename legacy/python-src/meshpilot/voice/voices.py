"""Профили голосов: Jarvis / Khan / Anime.

Маппинг на движки:
- Piper (offline ONNX): лучшие голоса с HuggingFace piper-voices
- Edge-TTS (online, Microsoft): ru-RU-DmitryNeural, en-GB-RyanNeural и т.д.
- Groq PlayAI (online): Fritz (мужской), Arista (женский), Ava
Для клонирования своих голосов в будущем: RVC / SoVITS / XTTS-v2 (см. docs/VOICES.md).
"""
from __future__ import annotations
from dataclasses import dataclass

@dataclass
class VoiceProfile:
    name: str
    label: str
    piper_voice: str
    edge_voice: str
    groq_voice: str
    rate: float = 1.0
    pitch: int = 0
    language: str = "ru"

PROFILES: dict[str, VoiceProfile] = {
    "jarvis": VoiceProfile(
        name="jarvis", label="Джарвис",
        piper_voice="en_GB-alan_medium", edge_voice="en-GB-RyanNeural",
        groq_voice="Fritz-PlayAI", rate=0.95, pitch=-2, language="en",
    ),
    "khan": VoiceProfile(
        name="khan", label="Хан",
        piper_voice="ru_RU-dmitry_medium", edge_voice="ru-RU-DmitryNeural",
        groq_voice="Arista-PlayAI", rate=0.82, pitch=-12, language="ru",
    ),
    "anime": VoiceProfile(
        name="anime", label="Мико",
        piper_voice="ru_RU-irina_medium", edge_voice="ru-RU-SvetlanaNeural",
        groq_voice="Ava-PlayAI", rate=1.12, pitch=14, language="ru",
    ),
}

def get_profile(name: str) -> VoiceProfile:
    return PROFILES.get(name, PROFILES["jarvis"])
