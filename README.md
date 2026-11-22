# MeshPilot

**Голосовой ассистент + оптимизатор CachyOS в одном проекте.**

Три голоса: **Джарвис** (Тони Старк), **Хан** (Metro), **Мико** (аниме-персонаж с моделькой, которая сидит на окнах и нижней панели). Подключение через **Groq + GigaChat + Hermes Agent** (каскад). Оптимизатор: чистка кэша всех приложений, удаление остатков, лучшие зеркала, DNS, **Zapret2** внутри.

## Возможности

### Голосовой ассистент
- **3 голоса:** Jarvis (британский, формальный), Khan (глубокий, хриплый, Metro), Anime (энергичная аниме-девушка)
- **STT:** faster-whisper локально + Groq Whisper fallback
- **TTS:** Piper (offline) → Edge-TTS → Groq PlayAI
- **Wake word:** openWakeWord («MeshPilot», «Hey Jarvis»), Silero VAD
- **Персонаж (Втубер-стиль):** Live2D (`.model3.json`) / VRM (`.vrm`) / PNG fallback — overlay сидит на окнах и нижней панели, lip-sync, эмоции, физика волос, взгляд за курсором. Модели качай с Booth.pm / VRoid Hub / itch.io (см. [docs/AVATAR_MODELS.md](docs/AVATAR_MODELS.md))
- **LLM каскад:** Groq (Llama 3.3 70B, быстрый) → GigaChat (Sber, русский) → Hermes 3/4 локально (Ollama, офлайн/приватность) → OpenAI-compat
- **Агент:** Hermes-стиль ReAct + function calling, tools оптимизатора, долговременная память (SQLite)

### Оптимизатор CachyOS
- **Чистка:** pacman cache (paccache), orphan-пакеты, journal, tmp, ~/.cache, браузеры, flatpak/snap/docker, thumbnails, coredumps, корзина, остатки приложений
- **Зеркала:** rate-mirrors → reflector → собственный бенчмарк (latency+speed), запись mirrorlist
- **DNS:** бенчмарк 15 серверов (Cloudflare/Google/Quad9/AdGuard/Yandex...), установка через systemd-resolved/NetworkManager
- **Zapret2:** встроенная установка (Flowseal/zapret2), стратегии (auto/fake-split/multisplit/syndata), автоподбор перебором, systemd, логи
- **Система:** CPU governor, I/O scheduler, fstrim, swappiness, zram, gaming mode, health-check

## Быстрый старт (CachyOS)

```bash
git clone https://github.com/Knox1986X/MeshPilot.git
cd MeshPilot
bash scripts/install.sh
source .venv/bin/activate
cp .env.example .env   # GROQ_API_KEY, GIGACHAT_*
meshpilot gui
```

CLI:
```bash
meshpilot chat "почисти кэш и поставь лучшие зеркала"
meshpilot voice --voice jarvis
meshpilot optimize --dry-run
meshpilot mirrors --dry-run
meshpilot dns --dry-run
meshpilot zapret status
meshpilot health
```

## Структура

```
src/meshpilot/
  core/         brain (каскад LLM), agent (Hermes ReAct), memory, events, assistant
  voice/        stt (faster-whisper/Groq), tts (Piper/Edge/Groq), wakeword, vad, voices
  avatar/       character (Live2D/VRM/PNG), overlay (сидит на окнах), lip-sync, animations
  gui/          main_window, panel (нижняя панель), tray
  optimizer/    cleaner, mirrors, dns, zapret, system, health, tools (для агента)
  integrations/ groq, gigachat, hermes (Ollama), openai_compat
config/         default.yaml, voices.yaml, zapret_strategies.yaml
docs/           ARCHITECTURE, VOICES, ZAPRET, SETUP, IDEAS (70+ идей)
scripts/        install.sh, uninstall.sh, build.sh
```

## Документация
- [Архитектура](docs/ARCHITECTURE.md) — почему Groq+GigaChat+Hermes
- [Голоса](docs/VOICES.md) — Jarvis/Khan/Anime, клонирование
- [Zapret2](docs/ZAPRET.md) — установка и стратегии
- [Установка](docs/SETUP.md)
- [Идеи и roadmap](docs/IDEAS.md) — 70+ идей развития

## Лицензия
MIT
