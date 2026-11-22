"""Text-to-Speech: Piper (offline) -> Edge-TTS -> Groq PlayAI.

Piper: быстрый, офлайн, голоса ONNX. Скачай голос:
  https://huggingface.co/rhasspy/piper-voices
  en_GB-alan_medium.onnx, ru_RU-dmitry_medium.onnx, ru_RU-irina_medium.onnx
Edge-TTS: качественные нейронные голоса Microsoft, нужен интернет.
Groq: PlayAI голоса, нужен GROQ_API_KEY.
"""
from __future__ import annotations
import asyncio
import logging
import subprocess
import tempfile
from pathlib import Path

from meshpilot.config import config
from meshpilot.voice.voices import get_profile

log = logging.getLogger("meshpilot.tts")

class TTS:
    def __init__(self, voice: str | None = None, engine: str | None = None) -> None:
        self.voice_name = voice or config.get("app", "voice", default="jarvis")
        self.profile = get_profile(self.voice_name)
        self.engine = engine or config.get("voice", "tts", "engine", default="piper")

    def set_voice(self, name: str) -> None:
        self.voice_name = name
        self.profile = get_profile(name)

    async def speak(self, text: str) -> bytes:
        """Синтез в WAV-байты. Порядок: piper -> edge -> groq -> fallback (sine beep)."""
        if not text.strip():
            return b""
        errors = []
        for engine in [self.engine, "edge", "groq"]:
            try:
                if engine == "piper":
                    return await self._piper(text)
                if engine == "edge":
                    return await self._edge(text)
                if engine == "groq":
                    return await self._groq(text)
            except Exception as e:
                errors.append(f"{engine}: {e}")
                log.warning("TTS %s failed: %s", engine, e)
                continue
        log.error("Все TTS движки недоступны: %s", errors)
        return self._fallback_beep()

    async def _piper(self, text: str) -> bytes:
        voice_file = Path("assets/voices") / f"{self.profile.piper_voice}.onnx"
        if not voice_file.exists():
            # пробуем системный piper с именем голоса
            voice_file = Path.home() / ".local/share/piper" / f"{self.profile.piper_voice}.onnx"
        if not voice_file.exists():
            raise FileNotFoundError(f"Piper голос не найден: {voice_file}. Скачай с huggingface.co/rhasspy/piper-voices")
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            out = f.name
        # piper CLI: echo text | piper -m voice.onnx -f out.wav --sentence-silence
        proc = await asyncio.create_subprocess_exec(
            "piper", "-m", str(voice_file), "-f", out,
            "--sentence-silence", "0.2",
            stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        )
        await proc.communicate(text.encode("utf-8"))
        data = Path(out).read_bytes()
        Path(out).unlink(missing_ok=True)
        return data

    async def _edge(self, text: str) -> bytes:
        import edge_tts
        rate = f"{int((self.profile.rate - 1.0) * 100):+d}%"
        pitch = f"{self.profile.pitch:+d}Hz"
        communicate = edge_tts.Communicate(text, self.profile.edge_voice, rate=rate, pitch=pitch)
        chunks = []
        async for chunk in communicate.stream():
            if chunk["type"] == "audio":
                chunks.append(chunk["data"])
        if not chunks:
            raise RuntimeError("Edge-TTS вернул пустой аудио")
        # Edge отдаёт MP3; для единообразия конвертируем через ffmpeg если есть, иначе возвращаем как есть
        mp3 = b"".join(chunks)
        try:
            return await self._to_wav(mp3)
        except Exception:
            return mp3

    async def _groq(self, text: str) -> bytes:
        if not config.env.groq_api_key:
            raise ValueError("GROQ_API_KEY не задан")
        from meshpilot.integrations.groq_client import GroqClient
        return await GroqClient().tts(text, voice=self.profile.groq_voice)

    async def _to_wav(self, mp3: bytes) -> bytes:
        with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as f:
            f.write(mp3)
            mp3_path = f.name
        wav_path = mp3_path.replace(".mp3", ".wav")
        await asyncio.create_subprocess_exec(
            "ffmpeg", "-y", "-i", mp3_path, wav_path,
            stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.DEVNULL,
        )
        data = Path(wav_path).read_bytes()
        Path(mp3_path).unlink(missing_ok=True)
        Path(wav_path).unlink(missing_ok=True)
        return data

    @staticmethod
    def _fallback_beep() -> bytes:
        import wave, io
        buf = io.BytesIO()
        with wave.open(buf, "wb") as w:
            w.setnchannels(1); w.setsampwidth(2); w.setframerate(22050)
            w.writeframes(b"\x00\x00" * 22050)  # 1 сек тишины
        return buf.getvalue()

async def play_wav(wav_bytes: bytes) -> None:
    """Проигрывание через sounddevice/ffplay fallback."""
    if not wav_bytes:
        return
    try:
        import io
        import sounddevice as sd
        import soundfile as sf
        data, sr = sf.read(io.BytesIO(wav_bytes))
        sd.play(data, sr)
        sd.wait()
    except Exception:
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            f.write(wav_bytes)
            p = f.name
        await asyncio.create_subprocess_exec("ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet", p)
        Path(p).unlink(missing_ok=True)
