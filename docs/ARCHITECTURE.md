# MeshPilot — архитектура

```
┌─────────────────────────────────────────────────────────┐
│  GUI (PySide6): MainWindow | Overlay (персонаж) | Panel │
├─────────────────────────────────────────────────────────┤
│  Assistant: wake → VAD → STT → Agent → TTS → Avatar     │
├─────────────────────────────────────────────────────────┤
│  Agent (Hermes-стиль ReAct + function calling)          │
│  Brain: Groq → GigaChat → Hermes(Ollama) → OpenAI-compat│
├─────────────────────────────────────────────────────────┤
│  Optimizer: cleaner | mirrors | dns | zapret | system   │
├─────────────────────────────────────────────────────────┤
│  Memory (SQLite) | Events (pub/sub) | Config (YAML+env) │
└─────────────────────────────────────────────────────────┘
```

## Почему Groq + GigaChat + Hermes (каскад)

| Провайдер | Плюсы | Минусы | Роль |
|---|---|---|---|
| **Groq** (Llama 3.3 70B) | ~500 tok/s, Whisper+TTS в одном API, бесплатный tier | Нужен интернет, US/EU серверы | Основной облачный, быстрый |
| **GigaChat** (Sber) | Русский из коробки, RU-хостинг | OAuth сложнее, медленнее Groq | RU-альтернатива, гос-сети |
| **Hermes** (Ollama локально) | Офлайн, приватность, function calling, без подписки | Нужен GPU/CPU, модель 8B слабее 70B | Офлайн/приватный fallback, агент |
| **OpenAI-compat** | Любой провайдер | — | Универсальный fallback |

**Hermes Agent аналоги:** LangGraph, AutoGen, CrewAI — тяжёлые. MeshPilot использует свой лёгкий
агент поверх Hermes (ReAct + tools), без LangChain. Если нужен полноценный фреймворк —
подключается LangGraph как опциональный backend (см. IDEAS.md).

## Голосовой конвейер

- **Wake word:** openWakeWord (offline ONNX) — «MeshPilot», «Hey Jarvis»
- **VAD:** Silero VAD (точнее energy-based)
- **STT:** faster-whisper (локально, int8) → Groq Whisper fallback
- **TTS:** Piper (offline) → Edge-TTS → Groq PlayAI
- **Lip-sync:** RMS амплитуда → открытие рта

## Персонаж (Avatar)

- **Live2D Cubism** (.model3.json) — основной, лёгкий, анимированный
- **VRM** (.vrm) — 3D fallback (будущее)
- **PNG** — статичный fallback с blink/bounce
- Overlay: transparent, always-on-top, draggable, dock к краям, click-through для игр

## Оптимизатор CachyOS

- **cleaner:** pacman cache, orphans, journal, tmp, user cache, браузеры, flatpak/snap/docker, thumbnails, coredumps, trash, остатки приложений
- **mirrors:** rate-mirrors → reflector → собственный бенчмарк (latency+speed)
- **dns:** бенчмарк 15 серверов (latency+DoH+filtering) → systemd-resolved/NetworkManager
- **zapret:** Zapret2 (Flowseal) установка, стратегии, автоподбор перебором, systemd
- **system:** CPU governor, I/O scheduler, fstrim, swappiness, zram, gaming mode
- **health:** диск/RAM/процессы/journal/cache → рекомендации
