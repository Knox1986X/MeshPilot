# Только проверенные идентификаторы: нет произвольных URL, хеша из окружения или latest.
iso_digest() {
  case $1 in
    251129) echo 2ab25f72ce8fece890b440cf9e260fc3d57ae4f5fb5308086f670240de02da7b ;;
    260809) echo 959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c ;;
    *) echo "Неизвестный профиль CachyOS Live" >&2; return 2 ;;
  esac
}
