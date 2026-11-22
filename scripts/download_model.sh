#!/usr/bin/env bash
# Помощник скачивания Втубер-модели в assets/models/
# Booth.pm отдаёт .rar, остальные — .zip. Поддерживаем оба.
# Использование: bash scripts/download_model.sh <url> <имя>
set -euo pipefail
URL="${1:-}"
NAME="${2:-miko}"
if [ -z "$URL" ]; then
  echo "Использование: bash scripts/download_model.sh <url> [имя]"
  echo "Пример Booth: bash scripts/download_model.sh https://booth.pm/downloadables/... jane"
  echo "Источники: Booth.pm, itch.io, Gumroad, VRoid Hub (см. docs/AVATAR_MODELS.md)"
  exit 1
fi
mkdir -p assets/models/"$NAME"
# определяем расширение по URL
EXT="zip"
case "$URL" in *.rar*) EXT="rar" ;; *.7z*) EXT="7z" ;; esac
ARCHIVE="/tmp/$NAME.$EXT"
echo "Скачиваю $URL -> $ARCHIVE"
curl -L "$URL" -o "$ARCHIVE"
echo "Распаковываю в assets/models/$NAME/"
if [ "$EXT" = "rar" ]; then
  command -v unrar >/dev/null 2>&1 || { echo "Нужен unrar: sudo pacman -S unrar"; exit 1; }
  unrar x "$ARCHIVE" "assets/models/$NAME/"
elif [ "$EXT" = "7z" ]; then
  7z x "$ARCHIVE" -o"assets/models/$NAME/"
else
  unzip -o "$ARCHIVE" -d "assets/models/$NAME/"
fi
echo "Готово. Ищем модель:"
find "assets/models/$NAME/" -name "*.model3.json" -o -name "*.vrm" -o -name "*.png" | head -10
echo "Укажи в config/default.yaml: model: assets/models/$NAME/ (или путь к .model3.json)"
