# MeshPilot (Tauri)

**Голосовой ассистент + оптимизатор CachyOS на Rust + TypeScript (Tauri).**

Три голоса: **Джарвис** (Тони Старк), **Хан** (Metro), **Мико** (аниме, Live2D-модель Jane Doe сидит на рабочем столе). LLM: **Groq + GigaChat + Hermes** (Ollama). Оптимизатор: чистка, зеркала, DNS, **Zapret2**.

> Python-версия заархивирована в `legacy/` для референса.

## Быстрый старт (CachyOS)

```bash
# зависимости
sudo pacman -S --needed rustup nodejs npm webkit2gtk-4.1

# модель персонажа
cp -r assets/models/jane public/models/

# запуск
npm install
npm run tauri dev
```

Сборка:
```bash
npm run tauri build
```

## Структура

```
src/              React + TypeScript фронт (Dashboard, Chat, Avatar Live2D)
src-tauri/        Rust бэкенд (sysinfo метрики, pacman/zapret/dns команды, LLM HTTP)
public/vendor/    PixiJS + Cubism SDK (офлайн, без CDN)
public/models/    Live2D модели (Jane Doe и др.)
assets/models/    Оригиналы моделей + CREDITS
config/           default.yaml, voices.yaml, zapret_strategies.yaml
legacy/           Python-версия (архив)
```

## Документация
- [Архитектура](docs/ARCHITECTURE.md)
- [Голоса](docs/VOICES.md)
- [Zapret2](docs/ZAPRET.md)
- [Модели персонажа](docs/AVATAR_MODELS.md)
- [Идеи](docs/IDEAS.md)

## Лицензия
MIT
