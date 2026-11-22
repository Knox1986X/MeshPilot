"""Голосовой конвейер MeshPilot: wake -> STT -> Agent -> TTS -> Avatar.

Цикл:
1. Wake word «MeshPilot / Джарвис» (openWakeWord)
2. VAD + запись фразы
3. STT (faster-whisper / Groq Whisper)
4. Agent (Brain каскад + tools оптимизатора)
5. TTS (Piper / Edge / Groq) голосом Jarvis/Khan/Anime
6. Avatar: listening -> thinking -> speaking (lip-sync)
"""
from __future__ import annotations
import asyncio
import logging

from meshpilot.avatar.character import Character, AvatarState
from meshpilot.core.agent import build_default_agent
from meshpilot.core.events import bus
from meshpilot.voice.stt import STT, record_wav
from meshpilot.voice.tts import TTS, play_wav
from meshpilot.voice.wakeword import WakeWord

log = logging.getLogger("meshpilot.assistant")

class Assistant:
    def __init__(self, voice: str | None = None) -> None:
        self.voice = voice
        self.character = Character()
        self.stt = STT()
        self.tts = TTS(voice=voice)
        self.agent = build_default_agent()
        self.wake = WakeWord()
        self._running = False

    async def handle_text(self, text: str) -> str:
        """Текст -> ответ + озвучка."""
        self.character.set_state(AvatarState.THINKING)
        await bus.emit("avatar.state", AvatarState.THINKING)
        reply = await self.agent.run(text)
        self.character.set_state(AvatarState.SPEAKING)
        await bus.emit("avatar.state", AvatarState.SPEAKING)
        wav = await self.tts.speak(reply)
        await play_wav(wav)
        self.character.set_state(AvatarState.IDLE)
        await bus.emit("avatar.state", AvatarState.IDLE)
        return reply

    async def handle_audio_file(self, wav_path: str) -> str:
        self.character.set_state(AvatarState.LISTENING)
        text = await self.stt.transcribe_file(wav_path)
        log.info("STT: %s", text)
        if not text.strip():
            return "Не расслышал, повторите."
        return await self.handle_text(text)

    async def run_once(self, seconds: float = 5.0) -> str:
        """Одна итерация: запись -> STT -> ответ -> TTS."""
        tmp = "/tmp/meshpilot_input.wav"
        await asyncio.to_thread(record_wav, tmp, seconds)
        return await self.handle_audio_file(tmp)

    async def run_loop(self) -> None:
        """Бесконечный цикл с wake word (упрощённо: запись по таймеру)."""
        self._running = True
        log.info("MeshPilot слушает... (Ctrl+C для выхода)")
        while self._running:
            try:
                reply = await self.run_once(seconds=5.0)
                print(f"MeshPilot: {reply}")
            except KeyboardInterrupt:
                break
            except Exception as e:
                log.error("assistant loop: %s", e)
                await asyncio.sleep(1)

    def stop(self) -> None:
        self._running = False
