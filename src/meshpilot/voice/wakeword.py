"""Wake word: openWakeWord (offline). Модели: mesh-pilot, hey-jarvis, hey-meshpilot.

openWakeWord: https://github.com/dscripka/openWakeWord — ONNX, работает на CPU, кастомизация через training.
Альтернатива: Porcupine (Picovoice) — точнее, но ключ нужен.
"""
from __future__ import annotations
import logging

log = logging.getLogger("meshpilot.wakeword")

class WakeWord:
    def __init__(self, threshold: float = 0.5) -> None:
        self.threshold = threshold
        self._model = None

    def _load(self):
        if self._model is None:
            from openwakeword.model import Model
            self._model = Model(wakeword_models=["mesh-pilot", "hey-jarvis", "hey-meshpilot"])
        return self._model

    def listen_once(self, audio_chunk) -> str | None:
        """audio_chunk: numpy int16 16kHz. Возвращает имя wake word или None."""
        try:
            model = self._load()
            scores = model.predict(audio_chunk)
            for name, score in scores.items():
                if score >= self.threshold:
                    return name
        except Exception as e:
            log.warning("wakeword недоступен: %s", e)
        return None

def train_custom_wakeword() -> None:
    """Заглушка: обучение своей модели openWakeWord (см. docs/VOICES.md)."""
    raise NotImplementedError("Обучение wake word: см. https://github.com/dscripka/openWakeWord#training")
