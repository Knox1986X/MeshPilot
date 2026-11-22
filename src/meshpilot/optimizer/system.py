"""Системные оптимизации CachyOS: CPU governor, I/O scheduler, zram, kernel params, gaming.

CachyOS уже оптимизирован из коробки, но MeshPilot добавляет:
- CPU governor: performance для игр, powersave для батареи
- I/O scheduler: bfq для SSD, none для NVMe
- zram / swap tuning
- sysctl: swappiness, vfs_cache_pressure, dirty_ratio
- systemd: journal limits, tmpfiles, fstrim timer
- gaming: gamemode, mangohud, latency tuning
"""
from __future__ import annotations
import subprocess
from pathlib import Path

def _run(cmd: list[str], sudo: bool = False) -> tuple[int, str]:
    full = (["sudo", "-n"] + cmd) if sudo else cmd
    try:
        r = subprocess.run(full, capture_output=True, text=True, timeout=30)
        return r.returncode, (r.stdout + r.stderr).strip()
    except Exception as e:
        return 1, str(e)

def cpu_governor(mode: str = "performance", dry_run: bool = True) -> str:
    if dry_run:
        return f"Dry-run: CPU governor -> {mode} (echo {mode} | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor)"
    code, out = _run(["bash", "-c", f"echo {mode} | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"])
    return f"CPU governor {mode}: {out or 'ok'}"

def io_scheduler(scheduler: str = "bfq", dry_run: bool = True) -> str:
    if dry_run:
        return f"Dry-run: I/O scheduler -> {scheduler} (для NVMe: none, для SATA SSD: bfq)"
    code, out = _run(["bash", "-c", f"echo {scheduler} | sudo tee /sys/block/sda/queue/scheduler 2>/dev/null; echo {scheduler} | sudo tee /sys/block/nvme0n1/queue/scheduler 2>/dev/null; true"])
    return f"I/O scheduler {scheduler}: {out or 'ok'}"

def enable_fstrim(dry_run: bool = True) -> str:
    if dry_run:
        return "Dry-run: sudo systemctl enable --now fstrim.timer"
    code, out = _run(["systemctl", "enable", "--now", "fstrim.timer"], sudo=True)
    return f"fstrim: {out or 'ok'}"

def tune_swappiness(value: int = 10, dry_run: bool = True) -> str:
    if dry_run:
        return f"Dry-run: vm.swappiness={value} (для SSD+zram рекомендуется 10-20, не 60)"
    code, out = _run(["bash", "-c", f"echo 'vm.swappiness={value}' | sudo tee /etc/sysctl.d/99-meshpilot.conf && sudo sysctl -p /etc/sysctl.d/99-meshpilot.conf"])
    return f"swappiness={value}: {out or 'ok'}"

def tune_zram(dry_run: bool = True) -> str:
    if dry_run:
        return "Dry-run: проверка zram (zramctl), при отсутствии — установка zram-generator"
    code, out = _run(["zramctl"])
    if "zram" in out.lower():
        return f"zram активен:\n{out}"
    return "zram не найден. Установи: sudo pacman -S zram-generator (CachyOS обычно уже ставит)"

def gaming_mode(enabled: bool = True, dry_run: bool = True) -> str:
    if dry_run:
        return f"Dry-run: gaming mode {'ON' if enabled else 'OFF'} (gamemode, CPU performance, disable powersave)"
    parts = []
    if enabled:
        code, out = _run(["bash", "-c", "gamemoderun true 2>/dev/null; echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"])
        parts.append(f"gamemode+performance: {out or 'ok'}")
    else:
        code, out = _run(["bash", "-c", "echo powersave | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"])
        parts.append(f"powersave: {out or 'ok'}")
    return "\n".join(parts)

def system_report() -> str:
    """Краткий отчёт о системе."""
    import platform
    import psutil
    vm = psutil.virtual_memory()
    disk = psutil.disk_usage("/")
    lines = [
        f"OS: {platform.system()} {platform.release()} ({platform.machine()})",
        f"CPU: {psutil.cpu_count(logical=True)} потоков, нагрузка {psutil.cpu_percent(interval=0.5):.0f}%",
        f"RAM: {vm.used/1e9:.1f}/{vm.total/1e9:.1f} ГБ ({vm.percent:.0f}%), swap: {psutil.swap_memory().percent:.0f}%",
        f"Диск /: {disk.used/1e9:.1f}/{disk.total/1e9:.1f} ГБ ({disk.percent:.0f}%)",
    ]
    # CachyOS?
    try:
        osr = Path("/etc/os-release").read_text()
        for line in osr.splitlines():
            if line.startswith("PRETTY_NAME"):
                lines.append(line)
    except Exception:
        pass
    return "\n".join(lines)
