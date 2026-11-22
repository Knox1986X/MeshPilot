"""Speech-to-Text: faster-whisper (локально) + Groq Whisper fallback.

faster-whisper: CTranslate2, int8, работает на CPU и GPU, модель small/medium — баланс качества/скорости.
Groq Whisper: whisper-large-v3-turbo, ~10x быстрее realtime, нужен интернет и GROQ_API_KEY.
"""
from __future__ import annotations
import asyncio
import logging
import wave
from pathlib import Path

from meshpilot.config import config

log = logging.getLogger("meshpilot.stt")

class STT:
    def __init__(self, engine: str | None = None) -> None:
        self.engine = engine or config.get("voice", "stt", "engine", default="faster-whisper")
        self.model_name = config.get("voice", "stt", "model", default="small")
        self.language = config.get("voice", "stt", "language", default="ru")
        self._model = None

    def _load(self):
        if self._model is None:
            from faster_whisper import WhisperModel
            device = config.get("voice", "stt", "device", default="cpu")
            compute = config.get("voice", "stt", "compute_type", default="int8")
            log.info("Загружаю faster-whisper %s (%s/%s)", self.model_name, device, compute)
            self._model = WhisperModel(self.model_name, device=device, compute_type=compute)
        return self._model

    async def transcribe_file(self, wav_path: str | Path) -> str:
        if self.engine == "groq" and config.env.groq_api_key:
            from meshpilot.integrations.groq_client import GroqClient
            data = Path(wav_path).read_bytes()
            return await GroqClient().transcribe(data, language=self.language)
        # локальный faster-whisper
        def _run() -> str:
            model = self._load()
            segments, _info = model.transcribe(str(wav_path), language=self.language, beam_size=5)
            return "".join(s.text for s in segments).strip()
        return await asyncio.to_thread(_run)

    async def transcribe_bytes(self, wav_bytes: bytes) -> str:
        tmp = Path("/tmp/meshpilot_stt.wav")
        tmp.write_bytes(wav_bytes)
        try:
            return await self.transcribe_file(tmp)
        finally:
            tmp.unlink(missing_ok=True)

def record_wav(path: str, seconds: float = 5.0, samplerate: int = 16000) -> None:
    """Запись с микрофона через sounddevice (если доступен)."""
    import sounddevice as sd
    import soundfile as sf
    rec = sd.rec(int(seconds * samplerate), samplerate=samplerate, channels=1, dtype="int16")
    sd.wait()
    sf.write(path, rec, samplerate)
