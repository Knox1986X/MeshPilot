#!/usr/bin/env bash
# Нужен видимый X11/XWayland-клиент именно запущенного процесса, не просто совпавший заголовок.
set -euo pipefail
probe=${2:-xdotool}
"$1" &
app=$!
trap 'kill "$app" 2>/dev/null || true; wait "$app" 2>/dev/null || true' EXIT
timeout 45s "$probe" search --sync --all --onlyvisible --pid "$app" --name '^MeshPilot([[:space:]]|$)' >/dev/null
kill -0 "$app"
echo "Проверено: появилось нативное окно MeshPilot с PID запущенного процесса"
