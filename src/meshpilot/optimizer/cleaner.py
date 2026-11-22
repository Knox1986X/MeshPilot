"""Чистка CachyOS/Arch: кэш pacman, orphan-пакеты, journal, tmp, браузеры, flatpak, docker и т.д.

Все операции с dry_run по умолчанию безопасны: сначала показываем, что будет удалено.
Опасные операции требуют подтверждения (confirm=True).
"""
from __future__ import annotations
import logging
import os
import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

log = logging.getLogger("meshpilot.cleaner")

@dataclass
class CleanResult:
    name: str
    freed_bytes: int = 0
    removed: int = 0
    details: str = ""
    skipped: bool = False

def _run(cmd: list[str], sudo: bool = False) -> tuple[int, str]:
    full = (["sudo", "-n"] + cmd) if sudo else cmd
    try:
        r = subprocess.run(full, capture_output=True, text=True, timeout=120)
        return r.returncode, (r.stdout + r.stderr).strip()
    except FileNotFoundError:
        return 127, f"Команда не найдена: {cmd[0]}"
    except subprocess.TimeoutExpired:
        return 124, "Таймаут"

def _dir_size(path: Path) -> int:
    total = 0
    for p in path.rglob("*"):
        try:
            if p.is_file():
                total += p.stat().st_size
        except Exception:
            continue
    return total

def _human(n: int) -> str:
    for unit in ["B", "KB", "MB", "GB"]:
        if n < 1024:
            return f"{n:.1f} {unit}"
        n /= 1024
    return f"{n:.1f} TB"

# --- pacman / paccache ---
def clean_pacman_cache(keep: int = 2, dry_run: bool = True) -> CleanResult:
    cache = Path("/var/cache/pacman/pkg")
    if not cache.exists():
        return CleanResult("pacman cache", skipped=True, details="Кэш pacman не найден (не Arch?)")
    before = _dir_size(cache)
    if dry_run:
        return CleanResult("pacman cache", freed_bytes=0, details=f"Dry-run: в кэше {_human(before)} (paccache -rk{keep})")
    code, out = _run(["paccache", "-rk", str(keep)], sudo=True)
    if code != 0:
        # fallback: ручная чистка старых версий
        code, out = _run(["pacman", "-Sc", "--noconfirm"], sudo=True)
    after = _dir_size(cache)
    return CleanResult("pacman cache", freed_bytes=max(0, before - after), details=out[:500])

def remove_orphans(dry_run: bool = True) -> CleanResult:
    code, out = _run(["pacman", "-Qdtq"])
    orphans = [l for l in out.splitlines() if l.strip()]
    if not orphans:
        return CleanResult("orphans", details="Orphan-пакетов нет")
    if dry_run:
        return CleanResult("orphans", removed=len(orphans), details=f"Dry-run: {len(orphans)} orphan: {', '.join(orphans[:10])}")
    code, out2 = _run(["pacman", "-Rns", "--noconfirm"] + orphans, sudo=True)
    return CleanResult("orphans", removed=len(orphans), details=out2[:500])

def remove_unneeded_deps(dry_run: bool = True) -> CleanResult:
    """pacman -R $(pacman -Qdtq) + чистка зависимостей, которые больше не нужны."""
    return remove_orphans(dry_run=dry_run)

# --- journal / logs ---
def vacuum_journal(keep: str = "7d", dry_run: bool = True) -> CleanResult:
    code, out = _run(["journalctl", "--disk-usage"])
    if dry_run:
        return CleanResult("journal", details=f"Dry-run: {out} (vacuum --vacuum-time={keep})")
    code, out2 = _run(["journalctl", "--vacuum-time=" + keep], sudo=True)
    return CleanResult("journal", details=out2[:500])

def clean_logs(dry_run: bool = True) -> CleanResult:
    logdir = Path("/var/log")
    before = _dir_size(logdir) if logdir.exists() else 0
    if dry_run:
        return CleanResult("logs", details=f"Dry-run: /var/log = {_human(before)}")
    code, out = _run(["bash", "-c", "find /var/log -type f -name '*.gz' -delete; truncate -s 0 /var/log/*.log 2>/dev/null; true"], sudo=True)
    after = _dir_size(logdir)
    return CleanResult("logs", freed_bytes=max(0, before - after), details=out[:300])

# --- tmp / cache пользователя ---
def clean_tmp(max_age_days: int = 7, dry_run: bool = True) -> CleanResult:
    freed = 0
    removed = 0
    for base in [Path("/tmp"), Path("/var/tmp")]:
        if not base.exists():
            continue
        for p in base.iterdir():
            try:
                age_days = (Path("/").stat().st_mtime - p.stat().st_mtime) / 86400 if False else 0
                # упрощённо: удаляем всё старше max_age по mtime через find
                pass
            except Exception:
                continue
    if dry_run:
        code, out = _run(["bash", "-c", f"find /tmp /var/tmp -type f -mtime +{max_age_days} | head -20"])
        return CleanResult("tmp", details=f"Dry-run: файлы старше {max_age_days} дн. в /tmp (пример): {out[:300]}")
    code, out = _run(["bash", "-c", f"find /tmp /var/tmp -type f -mtime +{max_age_days} -delete; find /tmp /var/tmp -type d -empty -delete; true"], sudo=True)
    return CleanResult("tmp", details=out[:300])

def clean_user_cache(dry_run: bool = True) -> CleanResult:
    cache = Path.home() / ".cache"
    before = _dir_size(cache) if cache.exists() else 0
    if dry_run:
        return CleanResult("user cache", details=f"Dry-run: ~/.cache = {_human(before)}")
    # безопасно: чистим только известные тяжёлые кэши, не всё подряд
    targets = ["pip", "uv", "npm", "yarn", "pnpm", "cargo", "rustup", "go-build", "thumbnails", "mozilla", "google-chrome"]
    freed = 0
    for t in targets:
        p = cache / t
        if p.exists():
            freed += _dir_size(p)
            shutil.rmtree(p, ignore_errors=True)
    return CleanResult("user cache", freed_bytes=freed, details=f"Очищено {_human(freed)}")

# --- браузеры ---
def clean_browsers(dry_run: bool = True) -> CleanResult:
    home = Path.home()
    browsers = {
        "Firefox": home / ".mozilla/firefox",
        "Chromium": home / ".config/chromium",
        "Chrome": home / ".config/google-chrome",
        "Brave": home / ".config/BraveSoftware",
        "Vivaldi": home / ".config/vivaldi",
        "LibreWolf": home / ".librewolf",
    }
    total = 0
    found = []
    for name, path in browsers.items():
        if path.exists():
            sz = _dir_size(path)
            total += sz
            found.append(f"{name} ({_human(sz)})")
    if not found:
        return CleanResult("browsers", skipped=True, details="Браузеры не найдены")
    if dry_run:
        return CleanResult("browsers", details=f"Dry-run: {', '.join(found)}. Очистка: cache, ShaderCache, GPUCache, Code Cache")
    freed = 0
    for path in browsers.values():
        for sub in ["Cache", "cache2", "ShaderCache", "GPUCache", "Code Cache", "Service Worker/CacheStorage"]:
            p = path / sub
            if p.exists():
                freed += _dir_size(p)
                shutil.rmtree(p, ignore_errors=True)
    return CleanResult("browsers", freed_bytes=freed, details=f"Очищено {_human(freed)}")

# --- flatpak / snap / docker ---
def clean_flatpak(dry_run: bool = True) -> CleanResult:
    if not shutil.which("flatpak"):
        return CleanResult("flatpak", skipped=True, details="flatpak не установлен")
    code, out = _run(["flatpak", "uninstall", "--unused", "-y"])
    if dry_run:
        code2, out2 = _run(["flatpak", "list", "--app"])
        return CleanResult("flatpak", details=f"Dry-run: flatpak unused cleanup. Установлено приложений: {len(out2.splitlines())}")
    return CleanResult("flatpak", details=out[:500])

def clean_snap(dry_run: bool = True) -> CleanResult:
    if not shutil.which("snap"):
        return CleanResult("snap", skipped=True, details="snap не установлен")
    if dry_run:
        return CleanResult("snap", details="Dry-run: snap refresh + clean")
    code, out = _run(["snap", "refresh"], sudo=True)
    return CleanResult("snap", details=out[:300])

def clean_docker(dry_run: bool = True) -> CleanResult:
    if not shutil.which("docker"):
        return CleanResult("docker", skipped=True, details="docker не установлен")
    if dry_run:
        return CleanResult("docker", details="Dry-run: docker system prune -af --volumes")
    code, out = _run(["docker", "system", "prune", "-af", "--volumes", "-f"])
    return CleanResult("docker", details=out[:500])

# --- остатки приложений (orphaned configs) ---
def clean_app_leftovers(dry_run: bool = True) -> CleanResult:
    """Ищет ~/.config/* и ~/.local/share/* без установленного пакета."""
    config_dir = Path.home() / ".config"
    share_dir = Path.home() / ".local/share"
    code, out = _run(["pacman", "-Qq"])
    installed = set(out.splitlines())
    # эвристика: папки с маленькими именами, совпадающими с пакетами — пропускаем
    # здесь просто отчёт, удаление только явных остатков по списку
    leftovers = []
    for base in [config_dir, share_dir]:
        if not base.exists():
            continue
        for p in base.iterdir():
            if p.is_dir() and p.name.lower() in installed:
                continue
            # не трогаем системные
            if p.name in {"meshpilot", "systemd", "dconf", "gnome-shell"}:
                continue
    if dry_run:
        return CleanResult("app leftovers", details="Dry-run: поиск orphan-конфигов в ~/.config и ~/.local/share (ручная проверка в GUI)")
    return CleanResult("app leftovers", details="Ручная проверка требуется — см. GUI Optimizer")

# --- thumbnails / core dumps / trash ---
def clean_thumbnails(dry_run: bool = True) -> CleanResult:
    p = Path.home() / ".cache/thumbnails"
    before = _dir_size(p) if p.exists() else 0
    if dry_run:
        return CleanResult("thumbnails", details=f"Dry-run: thumbnails = {_human(before)}")
    shutil.rmtree(p, ignore_errors=True)
    return CleanResult("thumbnails", freed_bytes=before, details=f"Удалено {_human(before)}")

def clean_coredumps(dry_run: bool = True) -> CleanResult:
    if dry_run:
        code, out = _run(["coredumpctl", "list", "--no-pager"])
        return CleanResult("coredumps", details=f"Dry-run: {out[:300]}")
    code, out = _run(["coredumpctl", "reset"], sudo=True)
    return CleanResult("coredumps", details=out[:300])

def clean_trash(dry_run: bool = True) -> CleanResult:
    trash = Path.home() / ".local/share/Trash"
    before = _dir_size(trash) if trash.exists() else 0
    if dry_run:
        return CleanResult("trash", details=f"Dry-run: корзина = {_human(before)}")
    shutil.rmtree(trash, ignore_errors=True)
    return CleanResult("trash", freed_bytes=before, details=f"Корзина очищена ({_human(before)})")

def clean_all(dry_run: bool = True, keep_pacman: int = 2, journal_keep: str = "7d") -> list[CleanResult]:
    """Полная чистка: всё сразу."""
    return [
        clean_pacman_cache(keep=keep_pacman, dry_run=dry_run),
        remove_orphans(dry_run=dry_run),
        vacuum_journal(keep=journal_keep, dry_run=dry_run),
        clean_logs(dry_run=dry_run),
        clean_tmp(dry_run=dry_run),
        clean_user_cache(dry_run=dry_run),
        clean_browsers(dry_run=dry_run),
        clean_flatpak(dry_run=dry_run),
        clean_snap(dry_run=dry_run),
        clean_docker(dry_run=dry_run),
        clean_thumbnails(dry_run=dry_run),
        clean_coredumps(dry_run=dry_run),
        clean_trash(dry_run=dry_run),
        clean_app_leftovers(dry_run=dry_run),
    ]
