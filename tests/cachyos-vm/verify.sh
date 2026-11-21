#!/usr/bin/env bash
# Офлайн-проверка перед публикацией скачанного файла и отдельно перед запуском гостя.
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
source "$here/fixtures.sh"
(( $# == 2 )) || { echo "Нужны путь к ISO и профиль" >&2; exit 2; }
select_fixture "$2" || exit 2
iso=$(realpath -e -- "$1"); sig=$iso.sig
reject() { echo "::error file=tests/cachyos-vm/verify.sh::$1" >&2; exit 1; }
[[ -f $iso && -f $sig ]] || reject "Нужны обычный файл ISO и соседняя подпись .sig"
bytes=$(stat -Lc %s -- "$iso"); sig_bytes=$(stat -Lc %s -- "$sig")
(( bytes > 0 && bytes <= 5368709120 && sig_bytes > 0 && sig_bytes <= 65536 )) || reject "Недопустимый размер ISO или подписи"
digest=$(sha256sum < "$iso"); [[ ${digest%% *} == "$sha" ]] || reject "SHA-256 ISO не совпадает с профилем $2"
key=$here/iso-signing-key.asc; digest=$(sha256sum < "$key")
[[ ${digest%% *} == 7f17cd0469e2b986d6c11ad8481a67705a20c5f968b198321a603567c4005576 ]] || reject "Изменён открытый ключ"
work=$(mktemp -d); trap 'rm -rf -- "$work"' EXIT
gpg --no-options --batch --homedir "$work" --dearmor --output "$work/iso.gpg" "$key"
gpgv --homedir "$work" --keyring "$work/iso.gpg" --status-fd 1 "$sig" "$iso" > "$work/status" || reject "Подпись ISO не прошла проверку"
awk -v expected=882DCFE48E2051D48E2562ABF3B607488DB35A47 '$1 == "[GNUPG:]" && $2 == "VALIDSIG" && ($3 == expected || $NF == expected) {ok=1} END {exit !ok}' "$work/status" || reject "Отпечаток подписанта не совпал"
echo "::notice file=tests/cachyos-vm/verify.sh::Проверены SHA-256 и подпись CachyOS $edition $release"
