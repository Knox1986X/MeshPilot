# Только проверенные идентификаторы: нет произвольных URL, хеша из окружения или latest.
select_fixture() {
  edition=desktop; release=$1; kernel=linux-cachyos-lts
  case $1 in
    251129) sha=2ab25f72ce8fece890b440cf9e260fc3d57ae4f5fb5308086f670240de02da7b ;;
    260809) sha=959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c ;;
    handheld-260628)
      edition=handheld; release=260628; kernel=linux-cachyos-deckify
      sha=0567c61f21622e5b4b482a874c41076beeb9bd8c00f046fa40e5c8cfbe34d15f ;;
    *) echo "Неизвестный профиль CachyOS Live" >&2; return 2 ;;
  esac
  name=cachyos-$edition-linux-$release.iso
}

# Некоторые архивы возвращают документ HTML со статусом 200 даже для отсутствующего файла.
# Только положительно распознанный текстовый ответ — промах доставки. Не искать другое медиа
# после несовпадения хеша бинарного файла или ошибки проверки OpenPGP.
html_response() {
  head -c 1024 -- "$1" | LC_ALL=C grep -aEi '^[[:space:]]*<(!doctype[[:space:]]+html|html)([[:space:]>]|$)' >/dev/null
}
