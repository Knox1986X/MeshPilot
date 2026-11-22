"""Health-check: что тормозит систему, что можно почистить, рекомендации."""
from __future__ import annotations
import subprocess
from pathlib import Path

import psutil

def health_check() -> dict:
    vm = psutil.virtual_memory()
    disk = psutil.disk_usage("/")
    # топ процессов по RAM
    procs = sorted(psutil.process_iter(["pid", "name", "memory_percent"]),
                   key=lambda p: p.info.get("memory_percent") or 0, reverse=True)[:5]
    top = [f"{p.info['name']} ({p.info['memory_percent']:.1f}%)" for p in procs]
    # journal size
    try:
        r = subprocess.run(["journalctl", "--disk-usage"], capture_output=True, text=True, timeout=10)
        journal = r.stdout.strip()
    except Exception:
        journal = "unknown"
    # pacman cache
    cache = Path("/var/cache/pacman/pkg")
    cache_mb = 0
    if cache.exists():
        cache_mb = sum(f.stat().st_size for f in cache.iterdir() if f.is_file()) / 1e6
    issues = []
    if disk.percent > 85:
        issues.append(f"Диск переполнен: {disk.percent:.0f}% — нужна чистка")
    if vm.percent > 85:
        issues.append(f"RAM перегружена: {vm.percent:.0f}% — проверь процессы")
    if cache_mb > 2000:
        issues.append(f"Кэш pacman большой: {cache_mb:.0f} МБ — paccache -rk2")
    if not issues:
        issues.append("Критичных проблем нет")
    return {
        "disk_percent": disk.percent,
        "ram_percent": vm.percent,
        "top_processes": top,
        "journal": journal,
        "pacman_cache_mb": round(cache_mb),
        "issues": issues,
    }

def health_report() -> str:
    h = health_check()
    lines = [
        f"Диск: {h['disk_percent']:.0f}% | RAM: {h['ram_percent']:.0f}% | Кэш pacman: {h['pacman_cache_mb']} МБ",
        f"Journal: {h['journal']}",
        "Топ процессов: " + ", ".join(h["top_processes"]),
        "Проблемы:",
    ]
    lines.extend(f"  - {i}" for i in h["issues"])
    return "\n".join(lines)
