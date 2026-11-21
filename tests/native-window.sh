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
if [[ $# == 3 ]]; then
  # Только в выделенной VM. Временной предел действует и на пробуждение шины доступности.
  timeout 27s "$3" "$app" ram || exit 42
  timeout 27s "$3" "$app" cpu || exit 43
  timeout 27s "$3" "$app" root_fs || exit 44
  timeout 27s "$3" "$app" net || exit 45
  kill -0 "$app"
fi
