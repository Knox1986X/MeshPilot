#!/usr/bin/env bash
set -euo pipefail
echo "MeshPilot uninstall (конфиги и кэш остаются в ~/.local/share/meshpilot)"
rm -rf .venv
echo "Удалено venv. Для полного удаления: rm -rf ~/.local/share/meshpilot ~/.config/meshpilot"
