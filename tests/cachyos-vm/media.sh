#!/usr/bin/env bash
# Получение неизменяемой фикстуры; не монтирует и не запускает образ.
set -euo pipefail
trap 'echo "::error file=tests/cachyos-vm/media.sh,line=$LINENO::Сбой на строке $LINENO" >&2' ERR
here=$(cd -- "$(dirname -- "$0")" && pwd)
source "$here/fixtures.sh"
(( $# == 1 || $# == 2 )) || { echo "Укажите каталог и при необходимости выпуск ISO" >&2; exit 2; }
release=${2-260809}
sha=$(iso_digest "$release") || exit 2
readonly name=cachyos-desktop-linux-$release.iso
readonly signer=882DCFE48E2051D48E2562ABF3B607488DB35A47
dest=${1:?Нужен пустой каталог для ISO}
mkdir -p -- "$dest"
[[ ! -e $dest/$name && ! -e $dest/$name.sig ]] || { echo "Не перезаписываем имеющееся медиа" >&2; exit 2; }
work=$(mktemp -d -- "$dest/.media.XXXXXXXX"); trap 'rm -rf -- "$work"' EXIT
# Хеш из CachyOS/wiki, ключ из CachyOS/CachyOS-PKGBUILDS. Источники в docs/VM-SMOKE.md.
printf '%s  %s\n' 7f17cd0469e2b986d6c11ad8481a67705a20c5f968b198321a603567c4005576 "$here/iso-signing-key.asc" | sha256sum --check --status
mkdir -m 700 "$work/gpg"
gpg --no-options --batch --homedir "$work/gpg" --dearmor --output "$work/keyring.gpg" "$here/iso-signing-key.asc"
opts=(--fail --location --silent --show-error --proto '=https' --proto-redir '=https' --connect-timeout 12)
sf="https://downloads.sourceforge.net/project/cachyos-arch/gui-installer/desktop/$release"
bases=("https://iso.cachyos.org/desktop/$release" "https://mirror.cachyos.org/ISO/desktop/$release" "$sf")
if [[ $release == 251129 ]]; then
  # Архивный выпуск: сначала архив проекта, затем зеркала из объявления этого выпуска.
  bases=("$sf" "https://cdn77.cachyos.org/ISO/desktop/$release" "https://mirror.cachyos.org/ISO/desktop/$release"
         "https://us.cachyos.org/ISO/desktop/$release" "https://iso.cachyos.org/desktop/$release"
         "https://mirrors.tuna.tsinghua.edu.cn/cachyos/desktop/$release" "https://archlinux.gay/cachy/ISO/desktop/$release")
fi
fetch() {
  local file=$1 seconds=$2 bytes=$3 output=$4 base code size
  for base in "${bases[@]}"; do
    if ! code=$(curl "${opts[@]}" --max-time "$seconds" --max-filesize "$bytes" -w '%{http_code}' "$base/$file" -o "$output"); then
      echo "::notice file=tests/cachyos-vm/media.sh::$base/$file — ошибка передачи, HTTP $code"; continue
    fi
    size=$(stat -c %s "$output")
    if html_response "$output"; then
      echo "::notice file=tests/cachyos-vm/media.sh::$base/$file — получен HTML, HTTP $code, $size байт"; continue
    fi
    echo "::notice file=tests/cachyos-vm/media.sh::$base/$file — HTTP $code, $size байт; ещё не проверен"
    return 0
  done
  echo "::error file=tests/cachyos-vm/media.sh::Не удалось получить $file ни из одного заданного источника" >&2
  return 1
}
# Источники файла и подписи могут отличаться. Ошибка проверки не вызывает смены источника.
fetch "$name.sig" 25 65536 "$work/media.sig"
if [[ $release == 251129 ]]; then
  rc=0; python3 "$here/archive.py" "$work" || rc=$?
  case $rc in
    0) ;;
    75) fetch "$name" 840 5368709120 "$work/media.iso" ;;
    *) exit "$rc" ;;
  esac
else
  fetch "$name" 300 5368709120 "$work/media.iso"
fi
printf '%s  %s\n' "$sha" "$work/media.iso" | sha256sum --check
gpgv --homedir "$work/gpg" --keyring "$work/keyring.gpg" --status-fd 1 "$work/media.sig" "$work/media.iso" > "$work/status"
awk -v expected="$signer" '$1 == "[GNUPG:]" && $2 == "VALIDSIG" && ($3 == expected || $NF == expected) {ok=1} END {exit !ok}' "$work/status"
mv -- "$work/media.sig" "$dest/$name.sig"; mv -- "$work/media.iso" "$dest/$name"
echo "::notice file=tests/cachyos-vm/media.sh::Проверены SHA-256 и подпись CachyOS Desktop $release"
