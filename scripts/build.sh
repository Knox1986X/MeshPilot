#!/usr/bin/env bash
set -euo pipefail
python3 -m pip install build
python3 -m build
echo "Пакет собран в dist/"
