#!/usr/bin/env bash
# Получение неизменяемой фикстуры; не монтирует и не запускает образ.
set -euo pipefail
trap 'echo "::error file=tests/cachyos-vm/media.sh,line=$LINENO::Сбой на строке $LINENO" >&2' ERR
here=$(cd -- "$(dirname -- "$0")" && pwd)
readonly name=cachyos-desktop-linux-260809.iso
readonly sha=959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c
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
for base in https://iso.cachyos.org/desktop/260809 https://mirror.cachyos.org/ISO/desktop/260809; do
  # Меняем источник только после ошибки передачи. Нарушение целостности — стоп.
  curl "${opts[@]}" --max-time 30 --max-filesize 65536 "$base/$name.sig" -o "$work/media.sig" || continue
  curl "${opts[@]}" --max-time 300 --max-filesize 5368709120 "$base/$name" -o "$work/media.iso" || continue
  printf '%s  %s\n' "$sha" "$work/media.iso" | sha256sum --check
  gpgv --homedir "$work/gpg" --keyring "$work/keyring.gpg" --status-fd 1 "$work/media.sig" "$work/media.iso" > "$work/status"
  awk -v expected="$signer" '$1 == "[GNUPG:]" && $2 == "VALIDSIG" && ($3 == expected || $NF == expected) {ok=1} END {exit !ok}' "$work/status"
  mv -- "$work/media.sig" "$dest/$name.sig"; mv -- "$work/media.iso" "$dest/$name"
  echo "Проверены SHA-256 и подпись CachyOS Desktop 260809"; exit 0
done
echo "::error file=tests/cachyos-vm/media.sh::Не удалось получить подписанный ISO из двух заданных источников" >&2; exit 1
