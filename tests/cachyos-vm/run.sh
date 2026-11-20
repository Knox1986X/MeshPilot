#!/usr/bin/env bash
# Только локальная VM без сети, диска назначения, SSH, QGA и общих каталогов.
set -euo pipefail
trap 'echo "::error file=tests/cachyos-vm/run.sh,line=$LINENO::Сбой на строке $LINENO" >&2' ERR
here=$(cd -- "$(dirname -- "$0")" && pwd)
if (( EUID == 0 || $# != 3 )); then echo "Нужны обычный пользователь, проверенный ISO, CLI и GUI" >&2; exit 2; fi
iso=$(realpath -- "$1"); cli=$(realpath -- "$2"); desktop=$(realpath -- "$3")
[[ -r /dev/kvm && -w /dev/kvm ]] || { echo "::error::KVM недоступен обычному пользователю"; exit 1; }
[[ -x $cli && -x $desktop ]] || { echo "::error::Испытуемые бинарники не найдены"; exit 1; }
# Не доверять имени образа, в том числе при запуске вручную после скачивания.
printf '%s  %s\n' 959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c "$iso" | sha256sum --check
work=$(mktemp -d); trap 'rm -rf -- "$work"' EXIT
mkdir -p "$work/payload/observer"
install -m 755 "$cli" "$work/payload/meshpilot"
install -m 755 "$desktop" "$work/payload/meshpilot-desktop"
install -m 644 "$here/guest.py" "$work/payload/guest.py"
install -m 755 "$here/../native-window.sh" "$work/payload/native-window.sh"
install -m 755 "$(command -v xdotool)" "$work/payload/observer/xdotool"
# Зависимость наблюдателя, а не приложение: MeshPilot не получает LD_LIBRARY_PATH.
install -m 644 /usr/lib/x86_64-linux-gnu/libxdo.so.3 "$work/payload/observer/libxdo.so.3"
cat > "$work/payload/window-observer" <<'WRAPPER'
#!/bin/sh
export LD_LIBRARY_PATH=/run/meshpilot-smoke/observer
exec /run/meshpilot-smoke/observer/xdotool "$@"
WRAPPER
chmod 755 "$work/payload/window-observer"
nonce=$(python3 -c 'import secrets; print(secrets.token_hex(16))'); echo "$nonce" > "$work/payload/run-id"
xorriso -as mkisofs -quiet -R -J -V MESHPILOT_TEST -o "$work/suite.iso" "$work/payload"
xorriso -osirrox on -indev "$iso" -extract /arch/boot/x86_64/vmlinuz-linux-cachyos-lts "$work/kernel" -extract /arch/boot/x86_64/initramfs-linux-cachyos-lts.img "$work/initramfs"
uuid=$(blkid -p -n iso9660 -s UUID -o value "$iso"); [[ $uuid =~ ^[A-Za-z0-9-]+$ ]]
args=("archisobasedir=arch" "archisosearchuuid=$uuid" copytoram=n cow_spacesize=1G console=tty0 console=ttyS0,115200
      systemd.unit=graphical.target systemd.wants=kernel-command-line.service
      systemd.mount-extra=LABEL=MESHPILOT_TEST:/run/meshpilot-smoke:iso9660:ro,nosuid,nodev
      'systemd.run="/usr/bin/python3 /run/meshpilot-smoke/guest.py"' systemd.run_success_action=poweroff systemd.run_failure_action=poweroff)
# Оба CD read-only. У гостя нет сетевой карты или пишущего блочного диска.
rc=0
timeout --kill-after=10s 420s qemu-system-x86_64 -no-user-config -nodefaults -machine q35,accel=kvm -cpu host -smp 2 -m 5G \
  -smbios type=1,product=MeshPilot-smoke -display none -device virtio-vga -nic none -monitor none -no-reboot \
  -serial "file:$work/boot.log" -serial "file:$work/result.log" \
  -drive "file=$iso,format=raw,media=cdrom,readonly=on" -drive "file=$work/suite.iso,format=raw,media=cdrom,readonly=on" \
  -kernel "$work/kernel" -initrd "$work/initramfs" -append "${args[*]}" || rc=$?
# Диагностика отсутствия UART, без публикации полного журнала ОС.
if [[ ! -s $work/result.log ]]; then
  for hint in "Linux version" "Kernel panic" "emergency mode" "kernel-command-line.service" "Traceback" "Welcome to CachyOS"; do
    if grep -Fq "$hint" "$work/boot.log"; then echo "::notice file=tests/cachyos-vm/run.sh::Отметка загрузки: $hint"; fi
  done
fi
python3 "$here/verdict.py" "$work/result.log" "$nonce" "$rc"
