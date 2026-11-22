"""Персонаж MeshPilot: состояния, эмоции, анимации (Live2D/VRM/PNG).

Состояния: idle | listening | thinking | speaking | error | sleeping
Эмоции (Live2D expressions / VRM blendshapes): neutral | smile | sad | thinking | surprised
"""
from __future__ import annotations
from enum import Enum

class AvatarState(str, Enum):
    IDLE = "idle"
    LISTENING = "listening"
    THINKING = "thinking"
    SPEAKING = "speaking"
    ERROR = "error"
    SLEEPING = "sleeping"

class Character:
    def __init__(self, model_path: str = "assets/models/meshpilot.model3.json") -> None:
        self.model_path = model_path
        self.state = AvatarState.IDLE
        self.mood = "neutral"
        self._blink_timer = 0.0

    def set_state(self, state: AvatarState | str) -> None:
        self.state = AvatarState(state) if isinstance(state, str) else state
        # авто-эмоции по состоянию
        mood_map = {
            AvatarState.SPEAKING: "smile",
            AvatarState.LISTENING: "neutral",
            AvatarState.THINKING: "thinking",
            AvatarState.ERROR: "sad",
            AvatarState.SLEEPING: "sleepy",
            AvatarState.IDLE: "neutral",
        }
        self.mood = mood_map.get(self.state, "neutral")

    def set_mood(self, mood: str) -> None:
        self.mood = mood

    def tick(self, dt: float) -> dict:
        """Параметры анимации: {blink, bounce, mouth, tilt}."""
        self._blink_timer += dt
        blink = 1.0 if (self._blink_timer % 3.2) < 0.12 else 0.0
        import math
        t = self._blink_timer
        bounce = math.sin(t * 2.0) * 0.02 if self.state == AvatarState.IDLE else 0.0
        mouth = 0.5 + 0.5 * math.sin(t * 8.0) if self.state == AvatarState.SPEAKING else 0.0
        tilt = math.sin(t * 0.7) * 0.03
        return {"blink": blink, "bounce": bounce, "mouth": mouth, "tilt": tilt}
