#!/usr/bin/env bash
# Получение неизменяемой фикстуры; не монтирует и не запускает образ.
set -euo pipefail
trap 'echo "::error file=tests/cachyos-vm/media.sh,line=$LINENO::Сбой на строке $LINENO" >&2' ERR
here=$(cd -- "$(dirname -- "$0")" && pwd)
source "$here/fixtures.sh"
(( $# == 1 || $# == 2 )) || { echo "Укажите каталог и при необходимости выпуск ISO" >&2; exit 2; }
profile=${2-260809}; select_fixture "$profile" || exit 2
[[ $profile != 251129 ]] || { echo "Архив не скачиваем автоматически. Нужны готовый ISO 251129 и его .sig для verify.sh" >&2; exit 2; }
dest=${1:?Нужен пустой каталог для ISO}
mkdir -p -- "$dest"
[[ ! -e $dest/$name && ! -e $dest/$name.sig ]] || { echo "Не перезаписываем имеющееся медиа" >&2; exit 2; }
work=$(mktemp -d -- "$dest/.media.XXXXXXXX"); trap 'rm -rf -- "$work"' EXIT
opts=(--fail --location --silent --show-error --proto '=https' --proto-redir '=https' --connect-timeout 12)
sf="https://downloads.sourceforge.net/project/cachyos-arch/gui-installer/$edition/$release"
bases=("https://iso.cachyos.org/$edition/$release" "https://cdn77.cachyos.org/ISO/$edition/$release" "https://mirror.cachyos.org/ISO/$edition/$release" "$sf")
fetch() {
  local file=$1 seconds=$2 bytes=$3 output=$4 base code size
  for base in "${bases[@]}"; do
    if ! code=$(curl -q "${opts[@]}" --max-time "$seconds" --max-filesize "$bytes" -w '%{http_code}' "$base/$file" -o "$output"); then
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
if [[ $edition == handheld ]]; then
  # Сверяем закреплённый отпечаток файла также с данными дистрибутива, а не только каталога.
  fetch "$name.sha256" 25 4096 "$work/published.sha256"
  awk -v sha="$sha" -v name="$name" 'NF==2 && $1==sha && ($2==name || $2=="*"name) {ok++} END {exit !(NR==1 && ok==1)}' "$work/published.sha256"
fi
fetch "$name.sig" 25 65536 "$work/$name.sig"
fetch "$name" 300 5368709120 "$work/$name"
bash "$here/verify.sh" "$work/$name" "$profile"
# Даже при параллельном старте не затереть файл, появившийся после начальной проверки.
ln -- "$work/$name.sig" "$dest/$name.sig"
ln -- "$work/$name" "$dest/$name"
