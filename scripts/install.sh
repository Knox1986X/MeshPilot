#!/usr/bin/env bash
# MeshPilot — установка на CachyOS (Arch)
set -euo pipefail
echo "=== MeshPilot install (CachyOS/Arch) ==="

# 1. Системные зависимости
sudo pacman -Syu --noconfirm
sudo pacman -S --noconfirm --needed \
  python python-pip git make gcc nftables iptables \
  ffmpeg portaudio alsa-lib \
  rate-mirrors reflector paccache \
  zram-generator gamemode \
  qt6-base qt6-multimedia qt6-webengine

# 2. Python venv + зависимости (ядро всегда, голос/GUI опционально)
python3 -m venv .venv
source .venv/bin/activate
pip install -U pip
pip install -e .
# Голос (STT/TTS). На Python 3.14 openwakeword может не встать (tflite-runtime) — это нормально,
# wake word просто не будет работать, остальной голос работает.
pip install -e ".[voice]" || echo "WARN: voice extras не встали, ядро работает без них"
# GUI: PySide6 уже есть из pacman (python-pyside6), WebEngine — qt6-webengine.
# Если pacman-версии нет, поставь через pip:
# pip install PySide6 PySide6-WebEngine

# 3. Ollama + Hermes (локальный LLM)
if ! command -v ollama &>/dev/null; then
  curl -fsSL https://ollama.com/install.sh | sh
fi
ollama pull nous-hermes3:8b || echo "WARN: ollama pull failed (проверь интернет)"
systemctl --user enable --now ollama || sudo systemctl enable --now ollama || true

# 4. Piper голоса
mkdir -p assets/voices ~/.local/share/piper
echo "Скачай Piper голоса с https://huggingface.co/rhasspy/piper-voices :"
echo "  en_GB-alan_medium.onnx (+ .json) -> assets/voices/"
echo "  ru_RU-dmitry_medium.onnx (+ .json) -> assets/voices/"
echo "  ru_RU-irina_medium.onnx (+ .json) -> assets/voices/"

# 5. .env
if [ ! -f .env ]; then
  cp .env.example .env
  echo "Создан .env — заполни GROQ_API_KEY и GIGACHAT_*"
fi

# 6. Zapret2 (опционально, через MeshPilot)
echo "Zapret2 ставится через: meshpilot zapret install --no-dry-run"

echo "=== Готово. Запуск: source .venv/bin/activate && meshpilot gui ==="
