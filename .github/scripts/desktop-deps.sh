#!/usr/bin/env bash
# Только подготовка одноразового Ubuntu runner, не пользовательская машина.
set -euo pipefail
[[ ${GITHUB_ACTIONS:-} == true && $EUID != 0 ]]
packages=(libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev xvfb xauth xdotool dbus-x11)
case ${1:-desktop} in
  desktop) ;;
  vm) packages+=(qemu-system-x86 xorriso acl libatspi2.0-dev) ;;
  *) exit 2 ;;
esac
opts=(-o Acquire::Retries=1 -o Acquire::http::Timeout=20 -o Acquire::https::Timeout=20)
if [[ -s /etc/apt/sources.list.d/ubuntu.sources ]]; then
  opts+=(-o Dir::Etc::sourcelist=/etc/apt/sources.list.d/ubuntu.sources -o Dir::Etc::sourceparts=-)
fi
sudo apt-get "${opts[@]}" -o APT::Update::Error-Mode=any update
sudo apt-get "${opts[@]}" install -y --no-install-recommends "${packages[@]}"
