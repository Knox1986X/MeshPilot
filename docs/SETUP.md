# MeshPilot — установка на CachyOS

## Быстрый старт
```bash
git clone https://github.com/Knox1986X/MeshPilot.git
cd MeshPilot
bash scripts/install.sh
source .venv/bin/activate
cp .env.example .env   # заполни ключи
meshpilot gui
```

## Ключи (.env)
- `GROQ_API_KEY` — https://console.groq.com/keys (бесплатно)
- `GIGACHAT_CLIENT_ID/SECRET` — https://developers.sber.ru/
- Hermes: `ollama pull nous-hermes3:8b` (локально, без ключа)

## Piper голоса
Скачай ONNX с https://huggingface.co/rhasspy/piper-voices в `assets/voices/`:
- en_GB-alan_medium (Jarvis)
- ru_RU-dmitry_medium (Khan)
- ru_RU-irina_medium (Anime)

## CLI примеры
```bash
meshpilot chat "почисти кэш и поставь лучшие зеркала"
meshpilot voice --voice jarvis
meshpilot optimize --dry-run
meshpilot mirrors --dry-run
meshpilot dns --dry-run
meshpilot zapret status
meshpilot health
```

## Проверка
```bash
pytest -q
meshpilot --help
```
