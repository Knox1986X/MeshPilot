"""Простая шина событий (async pub/sub) для модулей MeshPilot."""
from __future__ import annotations
import asyncio
from collections import defaultdict
from typing import Any, Awaitable, Callable

Handler = Callable[[Any], Awaitable[None]]

class EventBus:
    def __init__(self) -> None:
        self._subs: dict[str, list[Handler]] = defaultdict(list)

    def on(self, event: str, handler: Handler) -> None:
        self._subs[event].append(handler)

    async def emit(self, event: str, payload: Any = None) -> None:
        for h in list(self._subs.get(event, [])):
            try:
                await h(payload)
            except Exception:
                pass

    def emit_sync(self, event: str, payload: Any = None) -> None:
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            asyncio.run(self.emit(event, payload))
        else:
            loop.create_task(self.emit(event, payload))

bus = EventBus()

# Стандартные события:
#  voice.wake, voice.heard, voice.speaking, voice.done
#  avatar.state (idle|listening|thinking|speaking|error)
#  optimizer.progress, optimizer.done
#  zapret.status, system.health
