#!/usr/bin/env bash
# Только на одноразовом runner с Xvfb и сессионной шиной.
set -euo pipefail
"$1" &
app=$!
trap 'kill "$app" 2>/dev/null || true' EXIT
timeout 45s xdotool search --sync --onlyvisible --name '^MeshPilot' >/dev/null
kill -0 "$app"
echo "Проверено: появилось нативное окно MeshPilot"
