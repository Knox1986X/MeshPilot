"""Voice Activity Detection: Silero VAD (ONNX, быстрый, точный)."""
from __future__ import annotations
import logging

log = logging.getLogger("meshpilot.vad")

class VAD:
    def __init__(self, threshold: float = 0.5) -> None:
        self.threshold = threshold
        self._model = None

    def is_speech(self, audio_float) -> bool:
        """audio_float: numpy float32 [-1, 1], 16kHz. True если есть речь."""
        try:
            import torch
            from silero_vad import load_silero_vad
            if self._model is None:
                self._model = load_silero_vad()
            with torch.no_grad():
                prob = self._model(torch.from_numpy(audio_float), 16000).item()
            return prob >= self.threshold
        except Exception as e:
            log.debug("Silero VAD недоступен, fallback energy: %s", e)
            # fallback: энергия сигнала
            try:
                import numpy as np
                return float(np.sqrt((audio_float ** 2).mean())) > 0.02
            except Exception:
                return False
